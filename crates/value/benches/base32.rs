use criterion::{
    black_box,
    criterion_group,
    criterion_main,
    Criterion,
};
use value::base32;

pub fn benchmark_base32(c: &mut Criterion) {
    let value = (0..23u8).collect::<Vec<_>>();
    c.bench_function("base32::encode", |b| {
        b.iter(|| base32::encode(black_box(&value[..])))
    });
    c.bench_function("base32::encode_into", |b| {
        b.iter(|| {
            let mut buf = [0; 40];
            base32::encode_into(&mut buf, black_box(&value[..]));
            buf
        })
    });
    let encoded = base32::encode(&value[..]);
    c.bench_function("base32::decode", |b| {
        b.iter(|| base32::decode(black_box(&encoded)))
    });
}

criterion_group!(benches, benchmark_base32);
criterion_main!(benches);
