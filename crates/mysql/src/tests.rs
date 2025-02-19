use std::{
    cmp,
    collections::BTreeSet,
    env,
    sync::Arc,
    time::Instant,
};

use common::{
    assert_obj,
    document::{
        CreationTime,
        ResolvedDocument,
    },
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
use runtime::prod::ProdRuntime;

use crate::{
    ConvexMySqlPool,
    MySqlOptions,
    MySqlPersistence,
    EXPECTED_TABLE_COUNT,
};

run_persistence_test_suite!(
    opts,
    crate::itest::new_db_opts().await?,
    MySqlPersistence::new(
        Arc::new(ConvexMySqlPool::new(
            &opts.url.clone(),
            true,
            Option::<ProdRuntime>::None,
        )?),
        opts.db_name.clone(),
        MySqlOptions {
            allow_read_only: false,
            version: PersistenceVersion::V5,
            use_prepared_statements: true,
        },
        ShutdownSignal::panic(),
    )
    .await?,
    MySqlPersistence::new(
        Arc::new(ConvexMySqlPool::new(
            &opts.url.clone(),
            true,
            Option::<ProdRuntime>::None,
        )?),
        opts.db_name.clone(),
        MySqlOptions {
            allow_read_only: true,
            version: PersistenceVersion::V5,
            use_prepared_statements: true,
        },
        ShutdownSignal::panic(),
    )
    .await?
);

mod raw_statements {

    use super::*;

    run_persistence_test_suite!(
        opts,
        crate::itest::new_db_opts().await?,
        MySqlPersistence::new(
            Arc::new(ConvexMySqlPool::new(
                &opts.url.clone(),
                true,
                Option::<ProdRuntime>::None,
            )?),
            opts.db_name.clone(),
            MySqlOptions {
                allow_read_only: false,
                version: PersistenceVersion::V5,
                use_prepared_statements: false,
            },
            ShutdownSignal::panic()
        )
        .await?,
        MySqlPersistence::new(
            Arc::new(ConvexMySqlPool::new(
                &opts.url.clone(),
                true,
                Option::<ProdRuntime>::None,
            )?),
            opts.db_name.clone(),
            MySqlOptions {
                allow_read_only: true,
                version: PersistenceVersion::V5,
                use_prepared_statements: false,
            },
            ShutdownSignal::panic(),
        )
        .await?
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_loading_locally() -> anyhow::Result<()> {
    let options = MySqlOptions {
        allow_read_only: false,
        version: PersistenceVersion::V5,
        use_prepared_statements: false,
    };
    let opts = crate::itest::new_db_opts().await?;
    let persistence = MySqlPersistence::new(
        Arc::new(ConvexMySqlPool::new(
            &opts.url.clone(),
            options.use_prepared_statements,
            Option::<ProdRuntime>::None,
        )?),
        opts.db_name,
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
    let options = MySqlOptions {
        allow_read_only: false,
        version: PersistenceVersion::V5,
        use_prepared_statements: false,
    };
    let opts = crate::itest::new_db_opts().await?;
    let persistence = MySqlPersistence::new(
        Arc::new(ConvexMySqlPool::new(
            &opts.url.clone(),
            options.use_prepared_statements,
            Option::<ProdRuntime>::None,
        )?),
        opts.db_name,
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
            .replace_value(assert_obj!("buf" => ConvexValue::try_from(buf.clone())?))?;
        batch.push(DocumentLogEntry {
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
    let opts = crate::itest::new_db_opts().await?;
    let options = MySqlOptions {
        allow_read_only: false,
        version: PersistenceVersion::V5,
        use_prepared_statements: false,
    };
    let p1 = Arc::new(
        MySqlPersistence::new(
            Arc::new(ConvexMySqlPool::new(
                &opts.url.clone(),
                options.use_prepared_statements,
                Option::<ProdRuntime>::None,
            )?),
            opts.db_name.clone(),
            options,
            ShutdownSignal::no_op(),
        )
        .await?,
    );

    let mut id_generator = TestIdGenerator::new();
    let table: TableName = str::parse("table")?;
    let doc_id = id_generator.user_generate(&table);
    id_generator.write_tables(p1.clone()).await?;

    let doc = ResolvedDocument::new(doc_id, CreationTime::ONE, ConvexObject::empty())?;

    // Holding lease -- can write.
    p1.write(
        vec![
            (DocumentLogEntry {
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
    let options = MySqlOptions {
        allow_read_only: false,
        version: PersistenceVersion::V5,
        use_prepared_statements: false,
    };
    let p2 = Arc::new(
        MySqlPersistence::new(
            Arc::new(ConvexMySqlPool::new(
                &opts.url.clone(),
                options.use_prepared_statements,
                Option::<ProdRuntime>::None,
            )?),
            opts.db_name,
            options,
            ShutdownSignal::no_op(),
        )
        .await?,
    );

    // New Persistence can write.
    p2.write(
        vec![
            (DocumentLogEntry {
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
                (DocumentLogEntry {
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

#[tokio::test(flavor = "multi_thread")]
async fn test_table_count() -> anyhow::Result<()> {
    let options = MySqlOptions {
        allow_read_only: false,
        version: PersistenceVersion::V5,
        use_prepared_statements: false,
    };
    let opts = crate::itest::new_db_opts().await?;
    let persistence = MySqlPersistence::new(
        Arc::new(ConvexMySqlPool::new(
            &opts.url.clone(),
            options.use_prepared_statements,
            Option::<ProdRuntime>::None,
        )?),
        opts.db_name,
        options,
        ShutdownSignal::panic(),
    )
    .await?;

    let table_count = persistence.get_table_count().await?;
    assert_eq!(
        table_count, EXPECTED_TABLE_COUNT,
        "Unexpected number of tables after INIT_SQL. Did you forget to update \
         EXPECTED_TABLE_COUNT?"
    );
    Ok(())
}
