// Run with: `cargo bench -p database --bench is_stale --features testing`

use std::{
    collections::HashSet,
    str::FromStr,
};

use anyhow::Result;
use common::{
    document_index_keys::{
        DocumentIndexKeys,
        SearchValueTokens,
    },
    index::IndexKey,
    interval::{
        End,
        Interval,
        StartIncluded,
    },
    paths::FieldPath,
    query::FilterValue,
    testing::TestIdGenerator,
    types::{
        IndexDescriptor,
        TableName,
        TabletIndexName,
        Timestamp,
    },
    value::{
        id_v6::DeveloperDocumentId,
        ConvexValue,
        TabletIdAndTableNumber,
    },
};
use compact_str::CompactString;
use criterion::{
    criterion_group,
    criterion_main,
    BenchmarkId,
    Criterion,
};
use database::{
    write_log::{
        new_write_log,
        DocumentIndexKeysUpdate,
        LogWriter,
        WriteSource,
    },
    ReadSet,
    TransactionReadSet,
};
use maplit::btreemap;
use search::{
    query::TextQueryTerm,
    FilterConditionRead,
    QueryReads as SearchQueryReads,
    TextQueryTermRead,
};
use tokio::runtime::Runtime;
use value::val;

fn create_test_setup() -> Result<(
    LogWriter,
    TestIdGenerator,
    TabletIdAndTableNumber,
    TableName,
)> {
    let mut id_generator = TestIdGenerator::new();
    let table_name: TableName = "test_table".parse()?;
    let table_id_and_number = id_generator.user_table_id(&table_name);

    let (_log_owner, _log_reader, log_writer) = new_write_log(Timestamp::must(1000));

    Ok((log_writer, id_generator, table_id_and_number, table_name))
}

fn create_write_log_with_standard_index_writes(num_writes: usize) -> Result<(LogWriter, ReadSet)> {
    let (mut log_writer, mut id_generator, table_id_and_number, table_name) = create_test_setup()?;

    let index_name = TabletIndexName::new(
        table_id_and_number.tablet_id,
        IndexDescriptor::new("by_value")?,
    )?;

    // Add writes to the log with random values
    for i in 0..num_writes {
        let id = id_generator.user_generate(&table_name);

        // Generate random values - some will be in the target range, but not all
        // Use pseudo-random generation to ensure reproducibility
        let should_be_in_target_range = i < num_writes / 2 && i % 7 != 0;
        let value = if should_be_in_target_range {
            // Values in target range (0-25)
            (i * 3) % 26
        } else {
            // Values outside target range (26-100)
            26 + ((i * 7) % 75)
        };

        let index_key = IndexKey::new(vec![val!(value as i64)], id.into());
        let document_keys =
            DocumentIndexKeys::with_standard_index_for_test(index_name.clone(), index_key);

        let writes = vec![(
            id,
            DocumentIndexKeysUpdate {
                id,
                old_document_keys: None,
                new_document_keys: Some(document_keys),
            },
        )];

        log_writer.append(
            Timestamp::must((1001 + i) as i32),
            writes.into(),
            WriteSource::unknown(),
        );
    }

    // Create a read set that will conflict with only some writes (values 0-25)
    let field_path: FieldPath = "value".parse()?;
    let mut reads = TransactionReadSet::new();
    // Use IndexKey encoding for interval bounds
    let start_key = IndexKey::new(vec![val!(0)], DeveloperDocumentId::MIN).to_bytes();
    let end_key = IndexKey::new(vec![val!(26)], DeveloperDocumentId::MIN).to_bytes();
    let target_interval = Interval {
        start: StartIncluded(start_key.into()),
        end: End::Excluded(end_key.into()),
    };
    reads.record_indexed_directly(index_name, vec![field_path].try_into()?, target_interval)?;
    let read_set = reads.into_read_set();

    Ok((log_writer, read_set))
}

fn create_write_log_with_search_index_writes(num_writes: usize) -> Result<(LogWriter, ReadSet)> {
    let (mut log_writer, mut id_generator, table_id_and_number, table_name) = create_test_setup()?;

    let index_name = TabletIndexName::new(
        table_id_and_number.tablet_id,
        IndexDescriptor::new("search_index")?,
    )?;

    // Random words for generating text content
    let random_words = vec![
        "apple",
        "banana",
        "cherry",
        "dog",
        "elephant",
        "forest",
        "guitar",
        "house",
        "island",
        "jungle",
        "kite",
        "lemon",
        "mountain",
        "notebook",
        "ocean",
        "piano",
        "queen",
        "river",
        "sunset",
        "table",
        "umbrella",
        "village",
        "window",
        "xylophone",
        "yellow",
        "zebra",
        "adventure",
        "beautiful",
        "creative",
        "delicious",
        "exciting",
        "fantastic",
        "gorgeous",
        "happy",
        "incredible",
        "joyful",
        "knowledge",
        "lovely",
        "magnificent",
        "natural",
        "outstanding",
        "peaceful",
        "quality",
        "remarkable",
        "stunning",
        "terrific",
        "unique",
        "wonderful",
        "amazing",
        "brilliant",
        "charming",
    ];

    // Add writes to the log
    for i in 0..num_writes {
        let id = id_generator.user_generate(&table_name);
        let search_field: FieldPath = "content".parse()?;

        // Generate random text content - some will contain "target" word, but not all
        // documents that match filter conditions
        let should_contain_target = i < num_writes / 2 && i % 7 != 0;
        let mut text_words = Vec::new();

        // Add 20-30 random words
        let num_words = 20 + (i % 10);
        for j in 0..num_words {
            let word_idx = (i * 7 + j * 3) % random_words.len(); // Pseudo-random selection
            text_words.push(random_words[word_idx]);
        }

        // If this document should contain the target, insert it randomly
        if should_contain_target {
            let insert_pos = (i * 3) % text_words.len();
            text_words.insert(insert_pos, "target");
        }

        let filter_values = btreemap! {
            "category".parse()? => FilterValue::from_search_value(Some(&ConvexValue::String(
                if i % 3 == 0 { "important" } else { "normal" }
                    .to_string()
                    .try_into()?,
            ))),
            "priority".parse()? => FilterValue::from_search_value(Some(&ConvexValue::Int64((i % 5 + 1) as i64))),
            "active".parse()? => FilterValue::from_search_value(Some(&ConvexValue::Boolean(i % 2 == 0))),
        };

        let text_words_unique: HashSet<CompactString> =
            text_words.into_iter().map(|a| a.into()).collect();
        let document_keys = DocumentIndexKeys::with_search_index_for_test_with_filters(
            index_name.clone(),
            search_field,
            SearchValueTokens::from(text_words_unique),
            filter_values,
        );

        let writes = vec![(
            id,
            DocumentIndexKeysUpdate {
                id,
                old_document_keys: None,
                new_document_keys: Some(document_keys),
            },
        )];

        log_writer.append(
            Timestamp::must((1001 + i) as i32),
            writes.into(),
            WriteSource::unknown(),
        );
    }

    // Create search reads that will conflict with the search writes
    let field_path: FieldPath = "content".parse()?;
    let mut reads = TransactionReadSet::new();

    // Create a search read with both text query and filter conditions
    let search_reads = SearchQueryReads::new(
        vec![TextQueryTermRead {
            field_path,
            term: TextQueryTerm::Exact("target".to_string()),
        }]
        .into(),
        vec![
            // Filter must clauses - documents must match these conditions
            FilterConditionRead::Must(
                FieldPath::from_str("category")?,
                FilterValue::from_search_value(Some(&ConvexValue::String(
                    "important".to_string().try_into()?,
                ))),
            ),
            FilterConditionRead::Must(
                FieldPath::from_str("active")?,
                FilterValue::from_search_value(Some(&ConvexValue::Boolean(true))),
            ),
        ]
        .into(),
    );

    reads.record_search(index_name, search_reads);
    let read_set = reads.into_read_set();

    Ok((log_writer, read_set))
}

fn bench_is_stale_standard_index(c: &mut Criterion) {
    let rt = Runtime::new().expect("Failed to create Tokio runtime");
    let mut group = c.benchmark_group("is_stale_standard_index");

    for num_writes in [10, 50, 100, 500, 1000] {
        let (log_writer, read_set) = rt.block_on(async {
            create_write_log_with_standard_index_writes(num_writes)
                .expect("Failed to create test setup")
        });

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("writes_{}", num_writes)),
            &num_writes,
            |b, &_num_writes| {
                b.to_async(&rt).iter(|| async {
                    log_writer
                        .is_stale(&read_set, Timestamp::must(1000), Timestamp::must(2000))
                        .unwrap()
                })
            },
        );
    }

    group.finish();
}

fn bench_is_stale_search_index(c: &mut Criterion) {
    let rt = Runtime::new().expect("Failed to create Tokio runtime");
    let mut group = c.benchmark_group("is_stale_search_index");

    for num_writes in [10, 50, 100, 500, 1000] {
        let (log_writer, read_set) = rt.block_on(async {
            create_write_log_with_search_index_writes(num_writes)
                .expect("Failed to create test setup")
        });

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("writes_{}", num_writes)),
            &num_writes,
            |b, &_num_writes| {
                b.to_async(&rt).iter(|| async {
                    log_writer
                        .is_stale(&read_set, Timestamp::must(1000), Timestamp::must(2000))
                        .unwrap()
                })
            },
        );
    }

    group.finish();
}

fn bench_is_stale_no_conflict(c: &mut Criterion) {
    let rt = Runtime::new().expect("Failed to create Tokio runtime");
    let mut group = c.benchmark_group("is_stale_no_conflict");

    for num_writes in [10, 50, 100, 500, 1000] {
        let (log_writer, read_set) = rt.block_on(async {
            let (log_writer, _) = create_write_log_with_standard_index_writes(num_writes)
                .expect("Failed to create test setup");

            // Create a read set that won't conflict (empty interval)
            let mut id_generator = TestIdGenerator::new();
            let table_id_and_number = id_generator.user_table_id(&"test_table".parse().unwrap());
            let index_name = TabletIndexName::new(
                table_id_and_number.tablet_id,
                IndexDescriptor::new("by_value").unwrap(),
            )
            .unwrap();

            let field_path: FieldPath = "value".parse().unwrap();
            let mut reads = TransactionReadSet::new();
            reads
                .record_indexed_directly(
                    index_name,
                    vec![field_path].try_into().unwrap(),
                    Interval::empty(),
                )
                .unwrap();
            let read_set = reads.into_read_set();

            (log_writer, read_set)
        });

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("writes_{}", num_writes)),
            &num_writes,
            |b, &_num_writes| {
                b.to_async(&rt).iter(|| async {
                    log_writer
                        .is_stale(&read_set, Timestamp::must(1000), Timestamp::must(2000))
                        .unwrap()
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_is_stale_standard_index,
    bench_is_stale_search_index,
    bench_is_stale_no_conflict
);
criterion_main!(benches);
