#![feature(try_blocks)]
#![feature(lazy_cell)]

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
    sync::LazyLock,
    time::Instant,
};

use common::{
    bootstrap_model::index::text_index::DeveloperSearchIndexConfig,
    document::{
        CreationTime,
        ResolvedDocument,
    },
    query::{
        InternalSearch,
        InternalSearchFilterExpression,
        SearchVersion,
    },
    types::{
        IndexName,
        Timestamp,
        WriteTimestamp,
    },
};
use divan::counter::BytesCount;
use search::{
    build_term_weights,
    query::CompiledQuery,
    MemorySearchIndex,
    TantivySearchIndexSchema,
};
use serde::Deserialize;
use value::{
    assert_obj,
    InternalId,
    ResolvedDocumentId,
    TableIdentifier,
    TableNumber,
    TabletId,
    TabletIdAndTableNumber,
};

// Comment this out if you don't need memory profiling.
#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

const MAX_LOAD_SIZE: usize = 4 << 20;

#[derive(Deserialize)]
struct SearchDocument {
    text: String,
}

#[derive(Deserialize)]
struct Query {
    name: String,
    query: String,
}

struct Dataset {
    schema: TantivySearchIndexSchema,
    loaded: BTreeMap<String, Vec<(InternalId, CreationTime, ResolvedDocument, usize)>>,

    indexes: BTreeMap<String, MemorySearchIndex>,
    queries: BTreeMap<String, CompiledQuery>,
}

impl Dataset {
    fn load(path: &str) -> anyhow::Result<Self> {
        let mut next_id = 0u64;
        let mut alloc_id = || {
            let mut result = [0; 16];
            result[0..8].copy_from_slice(&next_id.to_le_bytes()[..]);
            next_id += 1;
            InternalId(result)
        };

        let table_id = TabletIdAndTableNumber::new_for_test(
            TabletId(alloc_id()),
            TableNumber::try_from(123).expect("Could not create table number"),
        );
        let index_name: IndexName = "messages.by_body".parse()?;
        let index_name = index_name.map_table(&|_| Ok(table_id.tablet_id))?;
        let config = DeveloperSearchIndexConfig {
            search_field: "body".parse()?,
            filter_fields: BTreeSet::new(),
        };

        let schema = TantivySearchIndexSchema::new(&config);
        let datasets = ["tweets", "wikipedia", "gutenberg"];

        let mut loaded = BTreeMap::new();
        for dataset in datasets {
            let f = File::open(&format!("{path}/{dataset}.jsonl"))?;
            let f = BufReader::new(f);
            let mut documents = vec![];
            for line in f.lines() {
                let d: SearchDocument = serde_json::from_str(&line?)?;
                let size = d.text.len();
                let internal_id = alloc_id();
                let id = ResolvedDocumentId::new(
                    table_id.tablet_id,
                    table_id.table_number.id(internal_id),
                );
                let value = assert_obj!("body" => d.text);
                let creation_time = CreationTime::try_from(1.)?;
                let document = ResolvedDocument::new(id, creation_time, value)?;
                documents.push((internal_id, creation_time, document, size));
            }
            loaded.insert(dataset.to_string(), documents);
        }

        let f = File::open(format!("{path}/queries.jsonl"))?;
        let f = BufReader::new(f);
        let mut queries = vec![];
        for line in f.lines() {
            let q: Query = serde_json::from_str(&line?)?;
            queries.push(q);
        }
        queries.sort_by(|a, b| a.name.cmp(&b.name));
        let mut compiled = BTreeMap::new();
        for q in queries {
            let internal_search = InternalSearch {
                index_name: index_name.clone(),
                table_name: "messages".parse()?,
                filters: vec![InternalSearchFilterExpression::Search(
                    "body".parse()?,
                    q.query,
                )],
            };
            let (compiled_query, _) = schema.compile(&internal_search, SearchVersion::V1)?;
            compiled.insert(q.name, compiled_query);
        }

        let mut indexes = BTreeMap::new();
        for (name, documents) in &loaded {
            let mut index = MemorySearchIndex::new(WriteTimestamp::Committed(Timestamp::MIN));
            for (internal_id, creation_time, document, _) in documents {
                let terms = schema.index_into_terms(document).unwrap();
                index
                    .update(
                        *internal_id,
                        WriteTimestamp::Pending,
                        None,
                        Some((terms, *creation_time)),
                    )
                    .unwrap();
            }
            indexes.insert(name.clone(), index);
        }

        Ok(Dataset {
            schema,
            loaded,
            queries: compiled,
            indexes,
        })
    }
}

static DATASET: LazyLock<Dataset> = LazyLock::new(|| {
    let path = env::var("DATASET")
        .expect("Set the `DATASET` variable to point to the test dataset (https://www.dropbox.com/sh/f0q1o7tbfuissm8/AAAkB-JggUKL7KFCtl1nsRf1a?dl=0)");
    Dataset::load(&path).unwrap()
});

fn dataset_args() -> impl Iterator<Item = &'static str> {
    ["tweets", "wikipedia", "gutenberg"].into_iter()
}

#[divan::bench(
    args = dataset_args(),
    max_time = 10,
)]
fn load(bencher: divan::Bencher, dataset_name: &str) {
    let dataset = &*DATASET;
    let documents = &dataset.loaded[dataset_name];

    let mut to_load = Vec::new();
    let mut total_size = 0;
    for (internal_id, creation_time, document, size) in documents {
        total_size += size;
        if total_size > MAX_LOAD_SIZE {
            break;
        }
        let terms = dataset.schema.index_into_terms(document).unwrap();
        to_load.push((*internal_id, *creation_time, terms));
    }
    bencher.counter(BytesCount::new(total_size)).bench(|| {
        let mut index = MemorySearchIndex::new(WriteTimestamp::Committed(Timestamp::MIN));
        for (internal_id, creation_time, terms) in &to_load {
            index
                .update(
                    *internal_id,
                    WriteTimestamp::Pending,
                    None,
                    Some((terms.clone(), *creation_time)),
                )
                .unwrap();
        }
        index
    });
}

fn query_args() -> impl Iterator<Item = String> {
    let queries = ["common", "infrequent", "long", "nonexistent", "phrase"];
    dataset_args().flat_map(move |n| queries.into_iter().map(move |q| format!("{n}:{q}")))
}

#[divan::bench(
    args = query_args(),
    max_time = 2,
)]
fn query(bencher: divan::Bencher, name: &str) {
    let (dataset_name, query_name) = name.split_once(':').unwrap();
    let dataset = &*DATASET;
    let index = &dataset.indexes[dataset_name];
    let query = &dataset.queries[query_name];

    let snapshot_ts = Timestamp::MIN;
    let stats_diff = index
        .bm25_statistics_diff(
            snapshot_ts,
            &query.text_query.iter().map(|q| q.term().clone()).collect(),
        )
        .unwrap();
    let (shortlist, ids) = index.bound_and_evaluate_query_terms(&query.text_query);
    let term_list_query = index.build_term_list_bitset_query(query, &shortlist, &ids);
    let term_weights = build_term_weights(&shortlist, &ids, &term_list_query, stats_diff).unwrap();

    bencher.bench(|| index.query(snapshot_ts, &term_list_query, &ids, &term_weights));
}

fn main() {
    let start = Instant::now();
    LazyLock::force(&DATASET);
    println!("Loaded dataset in {:?}", start.elapsed());
    divan::main();
}
