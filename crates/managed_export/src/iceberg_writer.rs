use std::{
    collections::{
        BTreeMap,
        HashMap,
    },
    sync::Arc,
};

use anyhow::{
    ensure,
    Context,
    Result,
};
use iceberg::{
    io::{
        S3_ENDPOINT,
        S3_PATH_STYLE_ACCESS,
    },
    table::Table,
    transaction::{
        ApplyTransactionAction,
        Transaction,
    },
    Catalog,
    CatalogBuilder,
    ErrorKind,
    NamespaceIdent,
    TableCreation,
    TableIdent,
};
use iceberg_catalog_glue::{
    GlueCatalogBuilder,
    AWS_ACCESS_KEY_ID,
    AWS_REGION_NAME,
    AWS_SECRET_ACCESS_KEY,
    GLUE_CATALOG_PROP_URI,
    GLUE_CATALOG_PROP_WAREHOUSE,
};

use super::{
    append,
    change_log_schema,
    Change,
};

const ENCODING_PROPERTY: &str = "convex.porter.encoding";
const ENCODING: &str = "convex-change-log-json-v1";
const RETIRED_PROPERTY: &str = "convex.porter.retired";
// Glue lowercases names, so these record the exact source table.
const COMPONENT_PROPERTY: &str = "convex.porter.component";
const TABLE_PROPERTY: &str = "convex.porter.table";
const ROOT_COMPONENT: &str = "app";
// Glue and Athena limit database and table names to 255 bytes.
const MAX_NAME_BYTES: usize = 255;
// S3 keys are at most 1024 bytes; leave room for Iceberg's file names.
const MAX_LOCATION_BYTES: usize = 896;

pub struct S3Destination {
    pub bucket: String,
    pub region: String,
    pub endpoint_url: Option<String>,
    pub access_key_id: String,
    pub secret_access_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceTable {
    pub component: String,
    pub table: String,
}

pub struct IcebergChangeWriter {
    catalog: Arc<dyn Catalog>,
    namespace: NamespaceIdent,
    warehouse_uri: String,
    selected: Vec<SourceTable>,
    tables: BTreeMap<SourceTable, Table>,
}

impl IcebergChangeWriter {
    /// Registers the tables in the Glue Data Catalog of the destination's
    /// account and region, with the same credentials. The caller owns the
    /// whole database. `selected` lists every source table the sync selects;
    /// a name that is too long or shared, such as `Users` and `users`, fails
    /// here before anything is dropped. When `fresh`, drops the database's
    /// tables first; dropping touches only Glue, so it works after the export
    /// moves to a bucket the credentials can no longer read.
    pub async fn for_s3(
        namespace: &str,
        warehouse_uri: &str,
        destination: S3Destination,
        selected: Vec<SourceTable>,
        fresh: bool,
    ) -> Result<Self> {
        ensure!(
            warehouse_uri.starts_with(&format!("s3://{}/", destination.bucket)),
            "Warehouse does not belong to destination bucket"
        );
        let mut props = HashMap::from([
            (
                GLUE_CATALOG_PROP_WAREHOUSE.to_owned(),
                warehouse_uri.to_owned(),
            ),
            (AWS_REGION_NAME.to_owned(), destination.region),
            (AWS_ACCESS_KEY_ID.to_owned(), destination.access_key_id),
            (
                AWS_SECRET_ACCESS_KEY.to_owned(),
                destination.secret_access_key,
            ),
        ]);
        // Glue exists only in AWS, so a custom endpoint is a local mock serving
        // both S3 and Glue.
        if let Some(endpoint) = destination.endpoint_url {
            props.insert(GLUE_CATALOG_PROP_URI.to_owned(), endpoint.clone());
            props.insert(S3_ENDPOINT.to_owned(), endpoint);
            props.insert(S3_PATH_STYLE_ACCESS.to_owned(), "true".to_owned());
        }
        let catalog = GlueCatalogBuilder::default().load("porter", props).await?;
        Self::open(Arc::new(catalog), namespace, warehouse_uri, selected, fresh).await
    }

    async fn open(
        catalog: Arc<dyn Catalog>,
        namespace: &str,
        warehouse_uri: &str,
        selected: Vec<SourceTable>,
        fresh: bool,
    ) -> Result<Self> {
        ensure!(
            is_glue_name(namespace),
            "Invalid Glue database name {namespace:?}"
        );
        Self::check_names(warehouse_uri, &selected)?;
        let namespace = NamespaceIdent::new(namespace.to_owned());
        if !catalog.namespace_exists(&namespace).await?
            && let Err(error) = catalog.create_namespace(&namespace, HashMap::new()).await
        {
            // A concurrent attempt may have created it first.
            ensure!(
                catalog.namespace_exists(&namespace).await?,
                "Cannot create Glue database: {error}"
            );
        }
        let writer = Self {
            catalog,
            namespace,
            warehouse_uri: warehouse_uri.trim_end_matches('/').to_owned(),
            selected,
            tables: BTreeMap::new(),
        };
        if fresh {
            for ident in writer.catalog.list_tables(&writer.namespace).await? {
                writer.catalog.drop_table(&ident).await?;
            }
        }
        Ok(writer)
    }

    fn component(source: &SourceTable) -> String {
        if source.component.is_empty() {
            ROOT_COMPONENT.to_owned()
        } else {
            source.component.replace('/', "__")
        }
    }

    fn table_name(source: &SourceTable) -> Result<String> {
        let name = format!("{}__{}", Self::component(source), source.table).to_lowercase();
        ensure!(
            is_glue_name(&name),
            "Cannot name a Glue table for {source:?}: names must be at most {MAX_NAME_BYTES} bytes"
        );
        Ok(name)
    }

    /// Directories keep the exact Convex names, since S3 keys are
    /// case-sensitive.
    fn directory(source: &SourceTable) -> String {
        format!("{}/{}", Self::component(source), source.table)
    }

    fn location(warehouse_uri: &str, source: &SourceTable) -> String {
        format!(
            "{}/{}",
            warehouse_uri.trim_end_matches('/'),
            Self::directory(source)
        )
    }

    fn validate(&self, source: &SourceTable, table: &Table) -> Result<()> {
        let metadata = table.metadata();
        ensure!(
            table.identifier().name() == Self::table_name(source)?
                && metadata.location().trim_end_matches('/')
                    == Self::location(&self.warehouse_uri, source),
            "Iceberg table name or location does not match its source"
        );
        ensure!(
            metadata.current_schema().as_struct() == change_log_schema()?.as_struct()
                && metadata.default_partition_spec().is_unpartitioned(),
            "Iceberg table schema or partitioning mismatch"
        );
        Ok(())
    }

    fn check_names<'a>(
        warehouse_uri: &str,
        sources: impl IntoIterator<Item = &'a SourceTable>,
    ) -> Result<()> {
        let mut by_name = BTreeMap::new();
        let mut by_directory = BTreeMap::new();
        for source in sources {
            ensure!(
                Self::location(warehouse_uri, source).len() <= MAX_LOCATION_BYTES,
                "Cannot store {source:?}: its S3 location is longer than {MAX_LOCATION_BYTES} \
                 bytes"
            );
            for (key, claimed) in [
                (Self::table_name(source)?, &mut by_name),
                (Self::directory(source), &mut by_directory),
            ] {
                if let Some(other) = claimed.insert(key.clone(), source)
                    && other != source
                {
                    anyhow::bail!(
                        "Convex tables {other:?} and {source:?} both map to Iceberg table {key}"
                    );
                }
            }
        }
        Ok(())
    }

    async fn load_existing(&self, ident: &TableIdent) -> Result<Option<Table>> {
        let table = match self.catalog.load_table(ident).await {
            Ok(table) => table,
            Err(error) if error.kind() == ErrorKind::TableNotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        self.validate(&source_of(&table)?, &table)?;
        Ok(Some(table))
    }

    async fn create(&mut self, source: &SourceTable) -> Result<()> {
        let name = Self::table_name(source)?;
        let creation = TableCreation::builder()
            .name(name.clone())
            .location(Self::location(&self.warehouse_uri, source))
            .schema(change_log_schema()?)
            .properties(HashMap::from([
                (ENCODING_PROPERTY.to_owned(), ENCODING.to_owned()),
                (COMPONENT_PROPERTY.to_owned(), source.component.clone()),
                (TABLE_PROPERTY.to_owned(), source.table.clone()),
            ]))
            .build();
        let table = match self.catalog.create_table(&self.namespace, creation).await {
            Ok(table) => table,
            // A concurrent attempt may have created it first.
            Err(error) => self
                .catalog
                .load_table(&TableIdent::new(self.namespace.clone(), name.clone()))
                .await
                .with_context(|| format!("Cannot create Iceberg table: {error}"))?,
        };
        ensure!(
            source_of(&table)? == *source,
            "Glue table {name} belongs to another source table"
        );
        self.validate(source, &table)?;
        self.tables.insert(source.clone(), table);
        Ok(())
    }

    /// Requires one writer per namespace at a time. An `iceberg` commit rebases
    /// onto the table it reloads by name, so an append that overlaps another
    /// writer's drop and recreate would land in the new table.
    pub async fn apply(
        &mut self,
        truncated: &[SourceTable],
        changes: Vec<(SourceTable, Change)>,
    ) -> Result<()> {
        Self::check_names(
            &self.warehouse_uri,
            self.selected
                .iter()
                .chain(truncated)
                .chain(changes.iter().map(|(source, _)| source)),
        )?;
        for source in truncated {
            let name = Self::table_name(source)?;
            // No two selected tables share a name, so any other table holding
            // this name is no longer selected, such as `Users` after the
            // deployment replaced it with `users`.
            let ident = TableIdent::new(self.namespace.clone(), name);
            if let Some(table) = self.load_existing(&ident).await? {
                self.catalog.drop_table(&ident).await?;
                self.tables.remove(&source_of(&table)?);
            }
            self.create(source).await?;
        }
        let mut by_table: BTreeMap<SourceTable, Vec<Change>> = BTreeMap::new();
        for (source, change) in changes {
            by_table.entry(source).or_default().push(change);
        }
        for (source, changes) in by_table {
            if !self.tables.contains_key(&source) {
                let ident = TableIdent::new(self.namespace.clone(), Self::table_name(&source)?);
                if let Some(table) = self.load_existing(&ident).await? {
                    ensure!(
                        source_of(&table)? == source,
                        "Glue table {} belongs to another source table",
                        ident.name()
                    );
                    self.tables.insert(source.clone(), table);
                } else {
                    self.create(&source).await?;
                }
            }
            let table = append(&*self.catalog, &self.tables[&source], &changes).await?;
            self.tables.insert(source, table);
        }
        Ok(())
    }

    /// Sets `convex.porter.retired` on owned tables absent from `selected` and
    /// clears it on the rest. Call this only at a consistent snapshot, when
    /// `selected` lists every source table.
    pub async fn retire_unselected(&mut self, selected: &[SourceTable]) -> Result<()> {
        // Retirement needs the full catalog, including tables untouched by this page.
        for ident in self.catalog.list_tables(&self.namespace).await? {
            if let Some(table) = self.load_existing(&ident).await? {
                self.tables.insert(source_of(&table)?, table);
            }
        }
        for (source, table) in &mut self.tables {
            let retired = !selected.contains(source);
            let marked = table
                .metadata()
                .properties()
                .get(RETIRED_PROPERTY)
                .is_some_and(|value| value == "true");
            if marked != retired {
                let tx = Transaction::new(table);
                let tx = tx
                    .update_table_properties()
                    .set(RETIRED_PROPERTY.to_owned(), retired.to_string())
                    .apply(tx)?;
                *table = tx.commit(&*self.catalog).await?;
            }
        }
        Ok(())
    }
}

fn source_of(table: &Table) -> Result<SourceTable> {
    let property = |key| table.metadata().properties().get(key).cloned();
    ensure!(
        property(ENCODING_PROPERTY).as_deref() == Some(ENCODING),
        "Glue table {} was not written by a Convex export",
        table.identifier()
    );
    Ok(SourceTable {
        component: property(COMPONENT_PROPERTY).context("Missing component property")?,
        table: property(TABLE_PROPERTY).context("Missing table property")?,
    })
}

fn is_glue_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_NAME_BYTES
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}
