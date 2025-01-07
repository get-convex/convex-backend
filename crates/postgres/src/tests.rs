use std::{
    cmp,
    collections::BTreeSet,
    env,
    sync::Arc,
    time::Instant,
};

use common::{
    document::{
        CreationTime,
        ResolvedDocument,
    },
    obj,
    persistence::{
        ConflictStrategy,
        DatabaseDocumentUpdate,
        Persistence,
    },
    run_persistence_test_suite,
    testing::{
        self,
        persistence_test_suite,
        TestIdGenerator,
    },
    types::{
        PersistenceVersion,
        TableName,
        Timestamp,
    },
    value::{
        ConvexObject,
        ConvexValue,
        Size,
    },
};
use futures::TryStreamExt;

use crate::{
    PostgresOptions,
    PostgresPersistence,
};

run_persistence_test_suite!(
    db,
    crate::itest::new_db_opts().await?,
    PostgresPersistence::new(
        &db,
        PostgresOptions {
            allow_read_only: false,
            version: PersistenceVersion::V5,
        }
    )
    .await?,
    PostgresPersistence::new(
        &db,
        PostgresOptions {
            allow_read_only: true,
            version: PersistenceVersion::V5,
        }
    )
    .await?
);

#[tokio::test(flavor = "multi_thread")]
async fn test_loading_locally() -> anyhow::Result<()> {
    let options = PostgresOptions {
        allow_read_only: false,
        version: PersistenceVersion::V5,
    };
    let persistence =
        PostgresPersistence::new(&crate::itest::new_db_opts().await?, options).await?; // need coverage on false too.

    let start = Instant::now();
    let reader = persistence.reader();
    let mut document_stream = reader.load_all_documents();
    let mut num_loaded = 0;
    let mut size_loaded = 0;
    let mut max_ts = None;
    while let Some((ts, _, maybe_doc)) = document_stream.try_next().await? {
        max_ts = cmp::max(max_ts, Some(ts));
        num_loaded += 1;
        if let Some(doc) = maybe_doc {
            size_loaded += doc.value().size();
        }
    }
    println!(
        "Loaded {} rows (size: {}) in {:?}",
        num_loaded,
        size_loaded,
        start.elapsed()
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_writing_locally() -> anyhow::Result<()> {
    let batch_size: usize = env::var("BATCH_SIZE")
        .unwrap_or_else(|_| "10".to_owned())
        .parse()?;
    let buf_size: usize = env::var("BUF_SIZE")
        .unwrap_or_else(|_| "10".to_owned())
        .parse()?;
    let options = PostgresOptions {
        allow_read_only: false,
        version: PersistenceVersion::V5,
    };
    let persistence =
        PostgresPersistence::new(&crate::itest::new_db_opts().await?, options).await?;

    let mut max_ts = None;
    {
        let reader = persistence.reader();
        let mut document_stream = reader.load_all_documents();
        while let Some((ts, ..)) = document_stream.try_next().await? {
            max_ts = cmp::max(max_ts, Some(ts));
        }
    }
    let mut next_ts = max_ts
        .map(|t| t.succ())
        .transpose()?
        .unwrap_or(Timestamp::MIN);

    let buf = vec![88; buf_size];
    let mut batch = vec![];
    for _ in 0..batch_size {
        let ts = next_ts;
        next_ts = next_ts.succ()?;

        let document = testing::generate::<ResolvedDocument>()
            .replace_value(obj!("buf" => ConvexValue::try_from(buf.clone())?)?)?;
        batch.push(DatabaseDocumentUpdate {
            ts,
            id: document.id_with_table_id(),
            value: Some(document),
            prev_ts: None,
        });
    }

    let start = Instant::now();
    persistence
        .write(batch, BTreeSet::new(), ConflictStrategy::Error)
        .await?;
    println!(
        "Wrote {} rows (payload size: {} bytes) in {:?}",
        batch_size,
        buf_size,
        start.elapsed()
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_lease_preempt() -> anyhow::Result<()> {
    let url = crate::itest::new_db_opts().await?;
    let options = PostgresOptions {
        allow_read_only: false,
        version: PersistenceVersion::default(),
    };
    let p1 = Arc::new(PostgresPersistence::new(&url, options).await?);

    let mut id_generator = TestIdGenerator::new();
    let table: TableName = str::parse("table")?;
    let doc_id = id_generator.user_generate(&table);
    id_generator.write_tables(p1.clone()).await?;

    let doc = ResolvedDocument::new(doc_id, CreationTime::ONE, ConvexObject::empty())?;

    // Holding lease -- can write.
    p1.write(
        vec![
            (DatabaseDocumentUpdate {
                ts: Timestamp::must(1),
                id: doc.id_with_table_id(),
                value: Some(doc.clone()),
                prev_ts: None,
            }),
        ],
        BTreeSet::new(),
        ConflictStrategy::Error,
    )
    .await?;

    // Acquire the lease from another Persistence.
    let options = PostgresOptions {
        allow_read_only: false,
        version: PersistenceVersion::V5,
    };
    let p2 = PostgresPersistence::new(&url, options).await?;

    // New Persistence can write.
    p2.write(
        vec![
            (DatabaseDocumentUpdate {
                ts: Timestamp::must(2),
                id: doc.id_with_table_id(),
                value: None,
                prev_ts: Some(Timestamp::must(1)),
            }),
        ],
        BTreeSet::new(),
        ConflictStrategy::Error,
    )
    .await?;

    // Old Persistence can read.
    // TODO(CX-1856) This is not good.
    let reader = p1.reader();
    let documents: Vec<_> = reader.load_all_documents().try_collect().await?;
    assert!(!documents.is_empty());

    // Old Persistence cannot write.
    let result = p1
        .write(
            vec![
                (DatabaseDocumentUpdate {
                    ts: Timestamp::must(3),
                    id: doc.id_with_table_id(),
                    value: Some(doc.clone()),
                    prev_ts: Some(Timestamp::must(1)),
                }),
            ],
            BTreeSet::new(),
            ConflictStrategy::Error,
        )
        .await;
    assert!(result.is_err());
    Ok(())
}
