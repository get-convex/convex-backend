use std::{
    collections::{
        BTreeMap,
        BTreeSet,
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
        S3_ACCESS_KEY_ID,
        S3_ENDPOINT,
        S3_PATH_STYLE_ACCESS,
        S3_REGION,
        S3_SECRET_ACCESS_KEY,
    },
    table::Table,
    transaction::{
        ApplyTransactionAction,
        Transaction,
    },
    Catalog,
    CatalogBuilder,
    NamespaceIdent,
    TableCreation,
    TableIdent,
};
use iceberg_catalog_rest::{
    RestCatalogBuilder,
    REST_CATALOG_PROP_URI,
};
use iceberg_storage_opendal::OpenDalStorageFactory;
use sha2::{
    Digest,
    Sha256,
};

use super::{
    append,
    change_log_schema,
    Change,
};

const NAMESPACE_PROPERTY: &str = "convex.porter.namespace";
const ENCODING_PROPERTY: &str = "convex.porter.encoding";
const ENCODING: &str = "convex-change-log-json-v1";
const RETIRED_PROPERTY: &str = "convex.porter.retired";
// Table names are hashes, so these name the source table for readers.
const COMPONENT_PROPERTY: &str = "convex.porter.component";
const TABLE_PROPERTY: &str = "convex.porter.table";

pub struct S3Destination {
    pub bucket: String,
    pub region: String,
    pub endpoint_url: Option<String>,
    pub access_key_id: String,
    pub secret_access_key: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceTable {
    pub component: String,
    pub table: String,
}

/// Writes Convex change pages into one namespace that it owns, one Iceberg
/// table per source table.
pub struct IcebergChangeWriter {
    catalog: Arc<dyn Catalog>,
    namespace: NamespaceIdent,
    warehouse_uri: String,
    tables: BTreeMap<String, Table>,
}

impl IcebergChangeWriter {
    pub async fn for_s3(
        catalog_url: &str,
        namespace: &str,
        warehouse_uri: &str,
        destination: S3Destination,
    ) -> Result<Self> {
        ensure!(
            warehouse_uri.starts_with(&format!("s3://{}/", destination.bucket)),
            "Warehouse does not belong to destination bucket"
        );
        let url = reqwest_iceberg::Url::parse(catalog_url)?;
        ensure!(
            matches!(url.scheme(), "http" | "https")
                && url.query().is_none()
                && url.fragment().is_none()
                && url.username().is_empty()
                && url.password().is_none(),
            "Invalid prototype catalog URL"
        );
        let mut roots = rustls::RootCertStore::empty();
        roots.add_parsable_certificates(rustls_native_certs::load_native_certs().certs);
        // Naming the provider keeps this client independent of a process default.
        let tls = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_safe_default_protocol_versions()?
        .with_root_certificates(roots)
        .with_no_client_auth();
        let client = reqwest_iceberg::Client::builder()
            .use_preconfigured_tls(tls)
            .build()?;
        let mut props = HashMap::from([
            (REST_CATALOG_PROP_URI.to_owned(), catalog_url.to_owned()),
            (S3_REGION.to_owned(), destination.region),
            (S3_ACCESS_KEY_ID.to_owned(), destination.access_key_id),
            (
                S3_SECRET_ACCESS_KEY.to_owned(),
                destination.secret_access_key,
            ),
        ]);
        if let Some(endpoint) = destination.endpoint_url {
            props.insert(S3_ENDPOINT.to_owned(), endpoint);
            props.insert(S3_PATH_STYLE_ACCESS.to_owned(), "true".to_owned());
        }
        let catalog = RestCatalogBuilder::default()
            .with_client(client)
            .with_storage_factory(Arc::new(OpenDalStorageFactory::S3 {
                customized_credential_load: None,
            }))
            .load("porter", props)
            .await?;
        Self::open(Arc::new(catalog), namespace, warehouse_uri).await
    }

    async fn open(catalog: Arc<dyn Catalog>, namespace: &str, warehouse_uri: &str) -> Result<Self> {
        ensure!(
            !namespace.is_empty()
                && namespace
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_'),
            "Invalid prototype namespace"
        );
        let namespace = NamespaceIdent::new(namespace.to_owned());
        if !catalog.namespace_exists(&namespace).await?
            && let Err(error) = catalog.create_namespace(&namespace, HashMap::new()).await
        {
            // A concurrent attempt may have created it first.
            ensure!(
                catalog.namespace_exists(&namespace).await?,
                "Cannot create prototype namespace: {error}"
            );
        }
        let mut writer = Self {
            catalog,
            namespace,
            warehouse_uri: warehouse_uri.trim_end_matches('/').to_owned(),
            tables: BTreeMap::new(),
        };
        for ident in writer.catalog.list_tables(&writer.namespace).await? {
            let table = writer.catalog.load_table(&ident).await?;
            writer.validate(&table)?;
            writer.tables.insert(ident.name().to_owned(), table);
        }
        Ok(writer)
    }

    fn table_name(source: &SourceTable) -> String {
        let mut hash = Sha256::new();
        hash.update((source.component.len() as u64).to_be_bytes());
        hash.update(source.component.as_bytes());
        hash.update(source.table.as_bytes());
        format!("t{:x}", hash.finalize())
    }

    fn location(&self, name: &str) -> String {
        format!("{}/{}/{name}", self.warehouse_uri, self.namespace.join("."))
    }

    fn validate(&self, table: &Table) -> Result<()> {
        let name = table.identifier().name();
        let metadata = table.metadata();
        let property = |key| metadata.properties().get(key).map(String::as_str);
        ensure!(
            property(NAMESPACE_PROPERTY) == Some(&*self.namespace.join("."))
                && property(ENCODING_PROPERTY) == Some(ENCODING),
            "Iceberg table ownership or encoding mismatch"
        );
        ensure!(
            metadata.location().trim_end_matches('/') == self.location(name),
            "Iceberg table location mismatch"
        );
        ensure!(
            metadata.current_schema().as_struct() == change_log_schema()?.as_struct()
                && metadata.default_partition_spec().is_unpartitioned(),
            "Iceberg table schema or partitioning mismatch"
        );
        Ok(())
    }

    async fn create(&mut self, source: &SourceTable) -> Result<()> {
        let name = &Self::table_name(source);
        let creation = TableCreation::builder()
            .name(name.to_owned())
            .location(self.location(name))
            .schema(change_log_schema()?)
            .properties(HashMap::from([
                (NAMESPACE_PROPERTY.to_owned(), self.namespace.join(".")),
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
                .load_table(&TableIdent::new(self.namespace.clone(), name.to_owned()))
                .await
                .with_context(|| format!("Cannot create Iceberg table: {error}"))?,
        };
        self.validate(&table)?;
        self.tables.insert(name.to_owned(), table);
        Ok(())
    }

    /// Applies one sync page. Each truncated table is dropped and recreated
    /// before the page's changes are appended, and a table is created the first
    /// time it appears.
    ///
    /// Requires one writer per namespace at a time. An `iceberg` commit rebases
    /// onto the table it reloads by name, so an append that overlaps another
    /// writer's drop and recreate would land in the new table.
    pub async fn apply(
        &mut self,
        truncated: &[SourceTable],
        changes: Vec<(SourceTable, Change)>,
    ) -> Result<()> {
        for source in truncated {
            let name = Self::table_name(source);
            if self.tables.contains_key(&name) {
                self.catalog
                    .drop_table(&TableIdent::new(self.namespace.clone(), name.clone()))
                    .await?;
                self.tables.remove(&name);
            }
            self.create(source).await?;
        }
        let mut by_table: BTreeMap<String, (SourceTable, Vec<Change>)> = BTreeMap::new();
        for (source, change) in changes {
            by_table
                .entry(Self::table_name(&source))
                .or_insert_with(|| (source, vec![]))
                .1
                .push(change);
        }
        for (name, (source, changes)) in by_table {
            if !self.tables.contains_key(&name) {
                self.create(&source).await?;
            }
            let table = append(&*self.catalog, &self.tables[&name], &changes).await?;
            self.tables.insert(name, table);
        }
        Ok(())
    }

    /// Sets `convex.porter.retired` on owned tables absent from `selected` and
    /// clears it on the rest. Call this only at a consistent snapshot, when
    /// `selected` lists every source table.
    pub async fn retire_unselected(&mut self, selected: &[SourceTable]) -> Result<()> {
        let selected: BTreeSet<_> = selected.iter().map(Self::table_name).collect();
        for (name, table) in &mut self.tables {
            let retired = !selected.contains(name);
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
