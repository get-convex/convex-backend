use std::{
    collections::HashMap,
    sync::Arc,
};

use arrow_iceberg::{
    Array,
    BooleanArray,
    Int64Array,
    StringArray,
};
use futures::TryStreamExt;
use iceberg::{
    memory::{
        MemoryCatalogBuilder,
        MEMORY_CATALOG_WAREHOUSE,
    },
    CatalogBuilder,
    NamespaceIdent,
    TableCreation,
};
use iceberg_storage_opendal::OpenDalStorageFactory;

use super::*;

pub(crate) async fn memory_catalog() -> Result<Arc<dyn Catalog>> {
    Ok(Arc::new(
        MemoryCatalogBuilder::default()
            .with_storage_factory(Arc::new(OpenDalStorageFactory::Memory))
            .load(
                "memory",
                HashMap::from([(
                    MEMORY_CATALOG_WAREHOUSE.to_owned(),
                    "memory://warehouse".to_owned(),
                )]),
            )
            .await?,
    ))
}

pub(crate) async fn read(table: &Table) -> Result<Vec<Change>> {
    let batches: Vec<_> = table
        .scan()
        .build()?
        .to_arrow()
        .await?
        .try_collect()
        .await?;
    let mut changes = vec![];
    for batch in batches {
        let column = |i: usize| batch.column(i).as_any();
        let ids = column(0).downcast_ref::<StringArray>().unwrap();
        let ts = column(1).downcast_ref::<Int64Array>().unwrap();
        let deleted = column(2).downcast_ref::<BooleanArray>().unwrap();
        let payloads = column(3).downcast_ref::<StringArray>().unwrap();
        for row in 0..batch.num_rows() {
            assert_eq!(deleted.value(row), payloads.is_null(row));
            changes.push(Change {
                id: ids.value(row).to_owned(),
                ts: ts.value(row).try_into()?,
                payload: (!payloads.is_null(row)).then(|| payloads.value(row).to_owned()),
            });
        }
    }
    changes.sort_by_key(|c| (c.id.clone(), c.ts));
    Ok(changes)
}

fn change(id: &str, ts: u64, payload: Option<&str>) -> Change {
    Change {
        id: id.to_owned(),
        ts,
        payload: payload.map(str::to_owned),
    }
}

#[tokio::test]
async fn appends_revisions_and_deletions_as_snapshots() -> Result<()> {
    let catalog = memory_catalog().await?;
    let namespace = NamespaceIdent::new("porter".to_owned());
    catalog.create_namespace(&namespace, HashMap::new()).await?;
    let table = catalog
        .create_table(
            &namespace,
            TableCreation::builder()
                .name("menu".to_owned())
                .schema(change_log_schema()?)
                .build(),
        )
        .await?;
    let first = [
        change("burger", 10, Some("{\"price\":10}")),
        change("soup", 10, Some("{\"price\":6}")),
    ];
    let table = append(&*catalog, &table, &first).await?;
    let second = [
        change("burger", 12, Some("{\"price\":12}")),
        change("soup", 13, None),
    ];
    let table = append(&*catalog, &table, &second).await?;
    assert_eq!(table.metadata().snapshots().count(), 2);
    assert_eq!(
        read(&table).await?,
        vec![
            first[0].clone(),
            second[0].clone(),
            first[1].clone(),
            second[1].clone(),
        ]
    );
    Ok(())
}
