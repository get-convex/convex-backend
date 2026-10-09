use std::collections::HashMap;

use iceberg::{
    Namespace,
    TableCommit,
};

use super::{
    super::tests::{
        change,
        memory_catalog,
        read,
    },
    *,
};

fn source(table: &str) -> SourceTable {
    component_source("", table)
}

fn component_source(component: &str, table: &str) -> SourceTable {
    SourceTable {
        component: component.to_owned(),
        table: table.to_owned(),
    }
}

async fn open(catalog: Arc<dyn Catalog>, fresh: bool) -> Result<IcebergChangeWriter> {
    open_selecting(catalog, vec![], fresh).await
}

async fn open_selecting(
    catalog: Arc<dyn Catalog>,
    selected: Vec<SourceTable>,
    fresh: bool,
) -> Result<IcebergChangeWriter> {
    IcebergChangeWriter::open(
        catalog,
        "convex_chess_wandering_fish_513",
        "memory://warehouse",
        selected,
        fresh,
    )
    .await
}

async fn changes(writer: &IcebergChangeWriter, table: &str) -> Result<Vec<Change>> {
    let ident = TableIdent::new(
        writer.namespace.clone(),
        IcebergChangeWriter::table_name(&source(table))?,
    );
    read(&writer.catalog.load_table(&ident).await?).await
}

/// Glue's `load_table` reports a missing table as `Unexpected`, not
/// `TableNotFound`.
#[derive(Debug)]
struct GlueLikeCatalog(Arc<dyn Catalog>);

#[async_trait::async_trait]
impl Catalog for GlueLikeCatalog {
    async fn list_namespaces(
        &self,
        parent: Option<&NamespaceIdent>,
    ) -> iceberg::Result<Vec<NamespaceIdent>> {
        self.0.list_namespaces(parent).await
    }

    async fn create_namespace(
        &self,
        namespace: &NamespaceIdent,
        properties: HashMap<String, String>,
    ) -> iceberg::Result<Namespace> {
        self.0.create_namespace(namespace, properties).await
    }

    async fn get_namespace(&self, namespace: &NamespaceIdent) -> iceberg::Result<Namespace> {
        self.0.get_namespace(namespace).await
    }

    async fn namespace_exists(&self, namespace: &NamespaceIdent) -> iceberg::Result<bool> {
        self.0.namespace_exists(namespace).await
    }

    async fn update_namespace(
        &self,
        namespace: &NamespaceIdent,
        properties: HashMap<String, String>,
    ) -> iceberg::Result<()> {
        self.0.update_namespace(namespace, properties).await
    }

    async fn drop_namespace(&self, namespace: &NamespaceIdent) -> iceberg::Result<()> {
        self.0.drop_namespace(namespace).await
    }

    async fn list_tables(&self, namespace: &NamespaceIdent) -> iceberg::Result<Vec<TableIdent>> {
        self.0.list_tables(namespace).await
    }

    async fn create_table(
        &self,
        namespace: &NamespaceIdent,
        creation: TableCreation,
    ) -> iceberg::Result<Table> {
        self.0.create_table(namespace, creation).await
    }

    async fn load_table(&self, table: &TableIdent) -> iceberg::Result<Table> {
        self.0.load_table(table).await.map_err(|error| {
            if error.kind() == iceberg::ErrorKind::TableNotFound {
                iceberg::Error::new(iceberg::ErrorKind::Unexpected, "Entity Not Found")
            } else {
                error
            }
        })
    }

    async fn drop_table(&self, table: &TableIdent) -> iceberg::Result<()> {
        self.0.drop_table(table).await
    }

    async fn purge_table(&self, table: &TableIdent) -> iceberg::Result<()> {
        self.0.purge_table(table).await
    }

    async fn table_exists(&self, table: &TableIdent) -> iceberg::Result<bool> {
        self.0.table_exists(table).await
    }

    async fn rename_table(&self, src: &TableIdent, dest: &TableIdent) -> iceberg::Result<()> {
        self.0.rename_table(src, dest).await
    }

    async fn register_table(
        &self,
        table: &TableIdent,
        metadata_location: String,
    ) -> iceberg::Result<Table> {
        self.0.register_table(table, metadata_location).await
    }

    async fn update_table(&self, commit: TableCommit) -> iceberg::Result<Table> {
        self.0.update_table(commit).await
    }
}

#[tokio::test]
async fn tables_missing_from_glue_are_created() -> Result<()> {
    let catalog: Arc<dyn Catalog> = Arc::new(GlueLikeCatalog(memory_catalog().await?));
    let mut writer = open(catalog, false).await?;
    writer
        .apply(
            &[source("menu")],
            vec![
                (source("menu"), change("burger", 10, Some("10"))),
                (source("orders"), change("o1", 11, Some("1"))),
            ],
        )
        .await?;
    assert_eq!(
        changes(&writer, "orders").await?,
        vec![change("o1", 11, Some("1"))]
    );
    Ok(())
}

#[tokio::test]
async fn truncates_recreate_tables_and_changes_append() -> Result<()> {
    let catalog = memory_catalog().await?;
    let mut writer = open(catalog.clone(), false).await?;
    writer
        .apply(
            &[source("menu"), source("empty")],
            vec![(source("menu"), change("burger", 10, Some("10")))],
        )
        .await?;
    writer
        .apply(&[], vec![(source("menu"), change("burger", 12, None))])
        .await?;
    assert_eq!(
        changes(&writer, "menu").await?,
        vec![change("burger", 10, Some("10")), change("burger", 12, None)]
    );
    assert!(changes(&writer, "empty").await?.is_empty());
    let properties = writer.tables[&source("menu")].metadata().properties();
    assert_eq!(properties[COMPONENT_PROPERTY], "");
    assert_eq!(properties[TABLE_PROPERTY], "menu");

    writer
        .apply(
            &[source("menu")],
            vec![(source("menu"), change("soup", 5, Some("6")))],
        )
        .await?;
    assert_eq!(
        changes(&writer, "menu").await?,
        vec![change("soup", 5, Some("6"))]
    );

    let mut reopened = open(catalog.clone(), false).await?;
    reopened
        .apply(
            &[],
            vec![(source("menu"), change("burger", 20, Some("12")))],
        )
        .await?;
    assert_eq!(
        changes(&reopened, "menu").await?,
        vec![
            change("burger", 20, Some("12")),
            change("soup", 5, Some("6"))
        ]
    );
    let restarted = open(catalog.clone(), true).await?;
    assert!(restarted.tables.is_empty());
    assert!(catalog.list_tables(&restarted.namespace).await?.is_empty());
    Ok(())
}

#[tokio::test]
async fn names_follow_components_and_keep_case_in_directories() -> Result<()> {
    let mut writer = open(memory_catalog().await?, false).await?;
    let node = component_source("aggregate/inner", "btreeNode");
    writer
        .apply(&[source("authRefreshTokens"), node.clone()], vec![])
        .await?;
    assert_eq!(writer.tables.len(), 2);
    for (source, name, location) in [
        (
            &node,
            "aggregate__inner__btreenode",
            "memory://warehouse/aggregate__inner/btreeNode",
        ),
        (
            &source("authRefreshTokens"),
            "app__authrefreshtokens",
            "memory://warehouse/app/authRefreshTokens",
        ),
    ] {
        let table = &writer.tables[source];
        assert_eq!(table.identifier().name(), name);
        assert_eq!(table.metadata().location(), location);
    }
    Ok(())
}

#[tokio::test]
async fn colliding_sources_fail_before_any_write() -> Result<()> {
    let mut writer = open(memory_catalog().await?, false).await?;
    let error = writer
        .apply(&[source("Users"), source("users")], vec![])
        .await
        .expect_err("a colliding page must be rejected");
    assert!(error.to_string().contains("both map to"), "{error:#}");
    assert!(writer.tables.is_empty());
    writer.apply(&[source("menu")], vec![]).await?;
    let error = open_selecting(
        writer.catalog.clone(),
        vec![source("Users"), source("users")],
        true,
    )
    .await
    .err()
    .expect("a colliding selection must be rejected");
    assert!(error.to_string().contains("both map to"), "{error:#}");
    assert_eq!(
        writer.catalog.list_tables(&writer.namespace).await?.len(),
        1
    );
    let deep = "s3://b/".to_owned() + &"p/".repeat(450);
    let error = IcebergChangeWriter::check_names(&deep, &[source("users")])
        .expect_err("an overlong location must be rejected");
    assert!(error.to_string().contains("longer than"), "{error:#}");
    Ok(())
}

#[tokio::test]
async fn a_new_table_replaces_a_leftover_with_the_same_name() -> Result<()> {
    let mut writer = open(memory_catalog().await?, false).await?;
    writer
        .apply(
            &[source("Users")],
            vec![(source("Users"), change("ada", 1, Some("{}")))],
        )
        .await?;
    for replacement in [source("users"), component_source("app", "users")] {
        writer = open(writer.catalog.clone(), false).await?;
        let error = writer
            .apply(
                &[],
                vec![(replacement.clone(), change("grace", 2, Some("{}")))],
            )
            .await
            .expect_err("a different source must truncate before reusing a Glue name");
        assert!(
            error.to_string().contains("belongs to another source"),
            "{error:#}"
        );
        writer
            .apply(
                std::slice::from_ref(&replacement),
                vec![(replacement.clone(), change("grace", 2, Some("{}")))],
            )
            .await?;
        assert_eq!(writer.tables.keys().collect::<Vec<_>>(), [&replacement]);
        assert_eq!(
            read(&writer.tables[&replacement]).await?,
            vec![change("grace", 2, Some("{}"))]
        );
    }
    Ok(())
}

#[tokio::test]
async fn retirement_follows_the_selection() -> Result<()> {
    let catalog = memory_catalog().await?;
    let mut writer = open(catalog, false).await?;
    writer
        .apply(&[source("menu"), source("orders")], vec![])
        .await?;
    let retired = |writer: &IcebergChangeWriter, table: &str| {
        writer.tables[&source(table)]
            .metadata()
            .properties()
            .get(RETIRED_PROPERTY)
            .cloned()
    };
    writer = open(writer.catalog.clone(), false).await?;
    writer.retire_unselected(&[source("menu")]).await?;
    assert_eq!(retired(&writer, "menu"), None);
    assert_eq!(retired(&writer, "orders").as_deref(), Some("true"));
    writer
        .retire_unselected(&[source("menu"), source("orders")])
        .await?;
    assert_eq!(retired(&writer, "orders").as_deref(), Some("false"));
    Ok(())
}

#[tokio::test]
async fn foreign_tables_are_checked_when_used_or_reconciled() -> Result<()> {
    let catalog = memory_catalog().await?;
    open(catalog.clone(), false).await?;
    catalog
        .create_table(
            &NamespaceIdent::new("convex_chess_wandering_fish_513".to_owned()),
            TableCreation::builder()
                .name("app__menu".to_owned())
                .schema(change_log_schema()?)
                .build(),
        )
        .await?;
    let mut writer = open(catalog, false).await?;
    writer
        .apply(
            &[],
            vec![(source("orders"), change("order", 1, Some("{}")))],
        )
        .await?;
    assert_eq!(changes(&writer, "orders").await?.len(), 1);

    let error = writer
        .apply(&[], vec![(source("menu"), change("burger", 1, Some("{}")))])
        .await
        .expect_err("appending to a foreign table must fail");
    assert!(error.to_string().contains("not written by"), "{error:#}");
    let error = writer
        .apply(&[source("menu")], vec![])
        .await
        .expect_err("truncating a foreign table must fail");
    assert!(error.to_string().contains("not written by"), "{error:#}");
    let error = writer
        .retire_unselected(&[])
        .await
        .expect_err("retirement must validate the full catalog");
    assert!(error.to_string().contains("not written by"), "{error:#}");
    Ok(())
}

#[tokio::test]
#[ignore = "Requires PORTER_AWS_MOCK_ENDPOINT (a moto server) and PORTER_AWS_MOCK_BUCKET"]
async fn glue_catalog_round_trip() -> Result<()> {
    let endpoint = std::env::var("PORTER_AWS_MOCK_ENDPOINT")?;
    let bucket = std::env::var("PORTER_AWS_MOCK_BUCKET")?;
    let namespace = format!("convex_test_{}", uuid::Uuid::new_v4().simple());
    let warehouse = format!("s3://{bucket}/tables");
    let open = || {
        IcebergChangeWriter::for_s3(
            &namespace,
            &warehouse,
            S3Destination {
                bucket: bucket.clone(),
                region: "us-east-1".to_owned(),
                endpoint_url: Some(endpoint.clone()),
                access_key_id: "test".to_owned(),
                secret_access_key: "test".to_owned(),
            },
            vec![source("menu")],
            false,
        )
    };
    let mut writer = open().await?;
    writer
        .apply(
            &[source("menu")],
            vec![(source("menu"), change("burger", 10, Some("10")))],
        )
        .await?;
    writer.retire_unselected(&[]).await?;
    let mut reopened = open().await?;
    reopened.retire_unselected(&[]).await?;
    let menu = &reopened.tables[&source("menu")];
    assert_eq!(read(menu).await?, vec![change("burger", 10, Some("10"))]);
    assert_eq!(menu.metadata().properties()[RETIRED_PROPERTY], "true");
    Ok(())
}
