use criterion::{
    criterion_group,
    criterion_main,
    Criterion,
};
use value::base32;

pub fn benchmark_base32(c: &mut Criterion) {
    let value = (0..23u8).collect::<Vec<_>>();
    c.bench_function("base32::encode", |b| b.iter(|| base32::encode(&value[..])));
    let encoded = base32::encode(&value[..]);
    c.bench_function("base32::decode", |b| b.iter(|| base32::decode(&encoded)));
}

criterion_group!(benches, benchmark_base32);
criterion_main!(benches);
