use std::collections::BTreeMap;

use common::types::{
    Timestamp,
    WriteTimestamp,
};
use criterion::{
    black_box,
    criterion_group,
    criterion_main,
    Criterion,
};
use rand::Rng;
use value::InternalId;
use vector::{
    CompiledVectorSearch,
    MemoryVectorIndex,
    QdrantDocument,
};

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut rng = rand::thread_rng();

    let n = 20000;
    let d = 1536;
    let k = 1024;

    let ts = Timestamp::must(1);

    let mut index = MemoryVectorIndex::new(WriteTimestamp::Committed(ts));
    let mut next_id = 1u128;

    for _ in 0..n {
        let id = InternalId(next_id.to_le_bytes());
        next_id += 1;
        let document = QdrantDocument {
            internal_id: id,
            vector: (0..d)
                .map(|_| rng.gen())
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
            filter_fields: BTreeMap::new(),
        };
        index
            .update(id, WriteTimestamp::Committed(ts), None, Some(document))
            .unwrap();
    }
    println!("size: {}", index.size());

    let search = CompiledVectorSearch {
        vector: (0..d)
            .map(|_| rng.gen())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
        limit: k,
        filter_conditions: BTreeMap::new(),
    };
    c.bench_function("query", |b| b.iter(|| index.query(ts, black_box(&search))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
