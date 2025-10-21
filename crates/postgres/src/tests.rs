use std::{
    assert_matches::assert_matches,
    cmp,
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
        DocumentLogEntry,
        Persistence,
    },
    run_persistence_test_suite,
    shutdown::ShutdownSignal,
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
use tokio_postgres::config::TargetSessionAttrs;

use crate::{
    ConnectError,
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
            schema: None,
            skip_index_creation: false,
            instance_name: "test".into(),
            multitenant: false,
        },
        ShutdownSignal::panic()
    )
    .await?
);

mod multitenant {
    use super::*;
    run_persistence_test_suite!(
        db,
        crate::itest::new_db_opts().await?,
        PostgresPersistence::new(
            &db,
            PostgresOptions {
                allow_read_only: false,
                version: PersistenceVersion::V5,
                schema: None,
                instance_name: "test".into(),
                multitenant: true,
                skip_index_creation: false,
            },
            ShutdownSignal::panic()
        )
        .await?
    );
}

mod with_non_default_schema {
    use super::*;
    run_persistence_test_suite!(
        db,
        crate::itest::new_db_opts().await?,
        PostgresPersistence::new(
            &db,
            PostgresOptions {
                allow_read_only: false,
                version: PersistenceVersion::V5,
                schema: Some("foobar".to_owned()),
                instance_name: "test".into(),
                multitenant: false,
                skip_index_creation: false,
            },
            ShutdownSignal::panic()
        )
        .await?
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_loading_locally() -> anyhow::Result<()> {
    let options = PostgresOptions {
        allow_read_only: false,
        version: PersistenceVersion::V5,
        schema: None,
        skip_index_creation: false,
        instance_name: "test".into(),
        multitenant: false,
    };
    let persistence = PostgresPersistence::new(
        &crate::itest::new_db_opts().await?,
        options,
        ShutdownSignal::panic(),
    )
    .await?; // need coverage on false too.

    let start = Instant::now();
    let reader = persistence.reader();
    let mut document_stream = reader.load_all_documents();
    let mut num_loaded = 0;
    let mut size_loaded = 0;
    let mut max_ts = None;
    while let Some(entry) = document_stream.try_next().await? {
        max_ts = cmp::max(max_ts, Some(entry.ts));
        num_loaded += 1;
        if let Some(doc) = entry.value {
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
        schema: None,
        skip_index_creation: false,
        instance_name: "test".into(),
        multitenant: false,
    };
    let persistence = PostgresPersistence::new(
        &crate::itest::new_db_opts().await?,
        options,
        ShutdownSignal::panic(),
    )
    .await?;

    let mut max_ts = None;
    {
        let reader = persistence.reader();
        let mut document_stream = reader.load_all_documents();
        while let Some(entry) = document_stream.try_next().await? {
            max_ts = cmp::max(max_ts, Some(entry.ts));
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
        batch.push(DocumentLogEntry {
            ts,
            id: document.id_with_table_id(),
            value: Some(document),
            prev_ts: None,
        });
    }

    let start = Instant::now();
    persistence
        .write(&batch, &[], ConflictStrategy::Error)
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
        schema: None,
        skip_index_creation: false,
        instance_name: "test".into(),
        multitenant: false,
    };
    let p1 = Arc::new(PostgresPersistence::new(&url, options, ShutdownSignal::no_op()).await?);

    let mut id_generator = TestIdGenerator::new();
    let table: TableName = str::parse("table")?;
    let doc_id = id_generator.user_generate(&table);
    id_generator.write_tables(p1.clone()).await?;

    let doc = ResolvedDocument::new(doc_id, CreationTime::ONE, ConvexObject::empty())?;

    // Holding lease -- can write.
    p1.write(
        &[(DocumentLogEntry {
            ts: Timestamp::must(1),
            id: doc.id_with_table_id(),
            value: Some(doc.clone()),
            prev_ts: None,
        })],
        &[],
        ConflictStrategy::Error,
    )
    .await?;

    // Acquire the lease from another Persistence.
    let options = PostgresOptions {
        allow_read_only: false,
        version: PersistenceVersion::V5,
        schema: None,
        skip_index_creation: false,
        instance_name: "test".into(),
        multitenant: false,
    };
    let p2 = PostgresPersistence::new(&url, options, ShutdownSignal::no_op()).await?;

    // New Persistence can write.
    p2.write(
        &[(DocumentLogEntry {
            ts: Timestamp::must(2),
            id: doc.id_with_table_id(),
            value: None,
            prev_ts: Some(Timestamp::must(1)),
        })],
        &[],
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
            &[(DocumentLogEntry {
                ts: Timestamp::must(3),
                id: doc.id_with_table_id(),
                value: Some(doc.clone()),
                prev_ts: Some(Timestamp::must(1)),
            })],
            &[],
            ConflictStrategy::Error,
        )
        .await;
    assert!(result.is_err());
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_read_only() -> anyhow::Result<()> {
    let options = PostgresOptions {
        allow_read_only: false,
        version: PersistenceVersion::default(),
        schema: None,
        skip_index_creation: false,
        instance_name: "test".into(),
        multitenant: false,
    };
    let url = crate::itest::new_db_opts().await?;
    let mut config: tokio_postgres::Config = url.parse()?;
    config.target_session_attrs(TargetSessionAttrs::ReadWrite);
    let pool = PostgresPersistence::create_pool(config)?;
    let load = |allow_read_only| {
        PostgresPersistence::with_pool(
            pool.clone(),
            PostgresOptions {
                allow_read_only,
                ..options.clone()
            },
            ShutdownSignal::panic(),
        )
    };
    load(false).await?;
    // Loading persistence should also succeed with allow_read_only=true
    load(true).await?;
    PostgresPersistence::set_read_only(pool.clone(), options.clone(), true).await?;
    // Now loading should fail...
    assert_matches!(load(false).await.err(), Some(ConnectError::ReadOnly));
    // ... unless allow_read_only=true
    load(true).await?;
    // ... until read-only is set to false again
    PostgresPersistence::set_read_only(pool.clone(), options.clone(), false).await?;
    load(false).await?;
    Ok(())
}
