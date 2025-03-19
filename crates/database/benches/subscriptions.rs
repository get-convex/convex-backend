#![feature(try_blocks)]

use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    env,
    fs::File,
    io::{
        BufRead,
        BufReader,
    },
    str::FromStr,
    time::Instant,
};

use common::{
    document::{
        CreationTime,
        PackedDocument,
        ResolvedDocument,
    },
    testing::TestIdGenerator,
    types::{
        GenericIndexName,
        IndexDescriptor,
        PersistenceVersion,
    },
};
use criterion::{
    criterion_group,
    criterion_main,
    BenchmarkId,
    Criterion,
};
use database::{
    subscription::SubscriptionManager,
    Token,
};
use humansize::{
    FormatSize,
    BINARY,
};
use itertools::Itertools;
use search::{
    convex_en,
    query::{
        FuzzyDistance,
        TextQueryTerm,
    },
};
use serde::Deserialize;
use value::{
    assert_obj,
    DeveloperDocumentId,
    FieldPath,
    InternalId,
    ResolvedDocumentId,
    TabletId,
    TabletIdAndTableNumber,
};

const MAX_LOAD_SIZE: usize = 4 << 20;
const TOTAL_SUBSCRIPTIONS: usize = 500;

#[derive(Deserialize)]
struct SearchDocument {
    text: String,
}

fn path() -> String {
    env::var("DATASET").expect(
        "Set the `DATASET` variable to point to the test dataset (ask Sujay for the dataset)",
    )
}

fn prefix_and_max_distances() -> Vec<(bool, FuzzyDistance)> {
    let mut result = vec![];
    for prefix in vec![true, false] {
        for distance in vec![FuzzyDistance::Zero, FuzzyDistance::One, FuzzyDistance::Two] {
            result.push((prefix, distance));
        }
    }
    result
}

fn load_datasets(
    table_id: TabletIdAndTableNumber,
    max_size: usize,
) -> anyhow::Result<BTreeMap<String, (Vec<PackedDocument>, Vec<String>)>> {
    let mut next_id = 0u64;
    let mut alloc_id = || {
        let mut result = [0; 16];
        result[0..8].copy_from_slice(&next_id.to_le_bytes()[..]);
        next_id += 1;
        InternalId(result)
    };

    let path = path();
    let start = Instant::now();
    println!("Loading from {path}...");
    let datasets = ["tweets", "wikipedia", "gutenberg"];

    let mut loaded = BTreeMap::new();
    let mut n = 0;
    let mut bytes = 0;
    let mut terms = 0;
    for dataset in datasets {
        let mut frequency_map: BTreeMap<String, u32> = BTreeMap::new();
        let f = File::open(format!("{path}/{dataset}.jsonl"))?;
        let f = BufReader::new(f);
        let mut documents = vec![];
        let mut m = 0;
        for line in f.lines() {
            if m > max_size {
                break;
            }
            let d: SearchDocument = serde_json::from_str(&line?)?;
            bytes += d.text.len();
            m += d.text.len();
            n += 1;
            let internal_id = alloc_id();
            let id = ResolvedDocumentId::new(
                table_id.tablet_id,
                DeveloperDocumentId::new(table_id.table_number, internal_id),
            );
            let tokenizer = convex_en();
            {
                let mut stream = tokenizer.token_stream(&d.text);
                while let Some(token) = stream.next() {
                    terms += 1;
                    *frequency_map.entry(token.text.clone()).or_default() += 1;
                }
            }
            let value = assert_obj!("body" => d.text);
            let creation_time = CreationTime::try_from(1.)?;
            let document = ResolvedDocument::new(id, creation_time, value)?;
            documents.push(PackedDocument::pack(document));
        }

        let terms_by_frequency: Vec<String> = frequency_map
            .into_iter()
            .sorted_by_key(|value| value.1)
            .map(|(key, _)| key)
            .collect();
        println!(
            "{dataset}: {}, {} docs, {terms} terms, {} unique terms",
            m.format_size(BINARY),
            documents.len(),
            terms_by_frequency.len()
        );
        loaded.insert(dataset.to_string(), (documents, terms_by_frequency));
    }

    println!(
        "Loaded {n} rows ({} bytes of text) in {:?}",
        bytes.format_size(BINARY),
        start.elapsed()
    );
    Ok(loaded)
}

fn create_subscription_token(
    tablet_id: TabletId,
    prefix: bool,
    max_distance: FuzzyDistance,
    token: String,
) -> Token {
    let index_name: GenericIndexName<TabletId> =
        GenericIndexName::new(tablet_id, IndexDescriptor::new("index").unwrap()).unwrap();

    Token::text_search_token(
        index_name,
        FieldPath::from_str("body").unwrap(),
        vec![TextQueryTerm::Fuzzy {
            token,
            prefix,
            max_distance,
        }],
    )
}

fn create_tokens(
    tablet_id: TabletId,
    terms_by_frequency: &Vec<String>,
    prefix: bool,
    max_distance: FuzzyDistance,
    count: usize,
) -> Vec<Token> {
    let total_unique_terms = terms_by_frequency.len();
    assert!(count <= total_unique_terms);
    terms_by_frequency
        .iter()
        .chunks(total_unique_terms / count)
        .into_iter()
        // Due to rounding errors we may end up with more chunks than required
        .take(count)
        .map(|chunk| {
            let token = chunk.into_iter().next().unwrap();
            create_subscription_token(tablet_id, prefix, max_distance, token.clone())
        })
        .collect::<Vec<_>>()
}

fn create_subscriptions(
    tablet_id: TabletId,
    terms_by_frequency: &Vec<String>,
    prefix: bool,
    max_distance: FuzzyDistance,
    count: usize,
) -> SubscriptionManager {
    let mut subscription_manager = SubscriptionManager::new_for_testing();
    let tokens = create_tokens(tablet_id, terms_by_frequency, prefix, max_distance, count);
    for token in tokens {
        // this drops the Subscription but in these tests we don't run the
        // worker that removes dropped subscriptions
        _ = subscription_manager.subscribe_for_testing(token).unwrap();
    }
    subscription_manager
}

fn bench_query(c: &mut Criterion) {
    let mut id_generator = TestIdGenerator::new();
    let table_name = id_generator.generate_table_name();
    let table_id = id_generator.user_table_id(&table_name);

    let datasets = load_datasets(table_id, MAX_LOAD_SIZE).unwrap();

    for (prefix, max_distance) in prefix_and_max_distances() {
        for (dataset, (data, terms_by_frequency)) in &datasets {
            let subscription_manager = create_subscriptions(
                table_id.tablet_id,
                terms_by_frequency,
                prefix,
                max_distance,
                TOTAL_SUBSCRIPTIONS,
            );

            let mut group = c.benchmark_group("subscriptions");

            group.throughput(criterion::Throughput::Elements(data.len() as u64));
            // Set the sample size higher when the cost isn't prohibitive.
            group.sample_size(if !prefix && max_distance != FuzzyDistance::Two {
                100
            } else {
                10
            });
            group.bench_with_input(
                BenchmarkId::from_parameter(format!(
                    "{TOTAL_SUBSCRIPTIONS}/{dataset}/{prefix}_{max_distance:?}"
                )),
                data,
                |b, documents| {
                    b.iter(|| {
                        for doc in documents {
                            let mut to_notify = BTreeSet::new();
                            subscription_manager.overlapping_for_testing(
                                doc,
                                &mut to_notify,
                                PersistenceVersion::V5,
                            );
                        }
                    })
                },
            );
            group.finish();
        }
    }
}

criterion_group!(benches, bench_query);
criterion_main!(benches);
