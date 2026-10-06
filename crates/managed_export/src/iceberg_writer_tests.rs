use super::{
    super::tests::{
        memory_catalog,
        read,
    },
    *,
};

fn source(table: &str) -> SourceTable {
    SourceTable {
        component: String::new(),
        table: table.to_owned(),
    }
}

fn change(id: &str, ts: u64, payload: Option<&str>) -> Change {
    Change {
        id: id.to_owned(),
        ts,
        payload: payload.map(str::to_owned),
    }
}

async fn changes(writer: &IcebergChangeWriter, table: &str) -> Result<Vec<Change>> {
    read(&writer.tables[&IcebergChangeWriter::table_name(&source(table))]).await
}

#[tokio::test]
async fn truncates_recreate_tables_and_changes_append() -> Result<()> {
    let catalog = memory_catalog().await?;
    let mut writer =
        IcebergChangeWriter::open(catalog.clone(), "porter", "memory://warehouse").await?;
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
    let properties = writer.tables[&IcebergChangeWriter::table_name(&source("menu"))]
        .metadata()
        .properties();
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

    let reopened = IcebergChangeWriter::open(catalog, "porter", "memory://warehouse").await?;
    assert_eq!(reopened.tables.len(), 2);
    Ok(())
}

#[tokio::test]
async fn retirement_follows_the_selection() -> Result<()> {
    let catalog = memory_catalog().await?;
    let mut writer = IcebergChangeWriter::open(catalog, "porter", "memory://warehouse").await?;
    writer
        .apply(&[source("menu"), source("orders")], vec![])
        .await?;
    let retired = |writer: &IcebergChangeWriter, table: &str| {
        writer.tables[&IcebergChangeWriter::table_name(&source(table))]
            .metadata()
            .properties()
            .get(RETIRED_PROPERTY)
            .cloned()
    };
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
async fn open_rejects_tables_it_does_not_own() -> Result<()> {
    let catalog = memory_catalog().await?;
    let namespace = NamespaceIdent::new("porter".to_owned());
    catalog.create_namespace(&namespace, HashMap::new()).await?;
    catalog
        .create_table(
            &namespace,
            TableCreation::builder()
                .name(IcebergChangeWriter::table_name(&source("menu")))
                .schema(change_log_schema()?)
                .build(),
        )
        .await?;
    let error = IcebergChangeWriter::open(catalog, "porter", "memory://warehouse")
        .await
        .err()
        .expect("a foreign table must be rejected");
    assert!(error.to_string().contains("ownership"), "{error:#}");
    Ok(())
}

#[tokio::test]
#[ignore = "Requires PORTER_AWS_MOCK_ENDPOINT (a moto server) and PORTER_AWS_MOCK_BUCKET"]
async fn glue_catalog_round_trip() -> Result<()> {
    let endpoint = std::env::var("PORTER_AWS_MOCK_ENDPOINT")?;
    let bucket = std::env::var("PORTER_AWS_MOCK_BUCKET")?;
    let namespace = format!("porter_{}", uuid::Uuid::new_v4().simple());
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
    let reopened = open().await?;
    let menu = &reopened.tables[&IcebergChangeWriter::table_name(&source("menu"))];
    assert_eq!(read(menu).await?, vec![change("burger", 10, Some("10"))]);
    assert_eq!(menu.metadata().properties()[RETIRED_PROPERTY], "true");
    Ok(())
}
