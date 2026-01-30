use criterion::{
    criterion_group,
    criterion_main,
    Criterion,
};
use serde_json::json;
use value::{
    json_deserialize,
    ConvexValue,
};

fn simple_value() -> serde_json::Value {
    json!({
        "a": 1,
        "b": 2,
        "c": 3,
        "d": 4,
        "e": 5,
        "f": 6,
        "g": 7,
        "h": 8,
        "i": 9,
        "j": 10,
    })
}
fn nested_value() -> serde_json::Value {
    let mut v = json!("hi");
    for _ in 0..64 {
        v = json!({ "nested": v });
    }
    v
}

pub fn benchmark_serialize(c: &mut Criterion) {
    let value = ConvexValue::try_from(simple_value()).unwrap();
    c.bench_function("to_json::simple", |b| b.iter(|| value.json_serialize()));
    let value = ConvexValue::try_from(nested_value()).unwrap();
    c.bench_function("to_json::nested", |b| b.iter(|| value.json_serialize()));
}

pub fn benchmark_deserialize(c: &mut Criterion) {
    let value = simple_value();
    c.bench_function("from_json_value::simple", |b| {
        b.iter(|| ConvexValue::try_from(value.clone()).unwrap())
    });
    let string = serde_json::to_string(&value).unwrap();
    c.bench_function("from_json::simple", |b| {
        b.iter(|| json_deserialize(&string).unwrap())
    });
    let value = nested_value();
    c.bench_function("from_json_value::nested", |b| {
        b.iter(|| ConvexValue::try_from(value.clone()).unwrap())
    });
    let string = serde_json::to_string(&value).unwrap();
    c.bench_function("from_json::nested", |b| {
        b.iter(|| json_deserialize(&string).unwrap())
    });
}

criterion_group!(benches, benchmark_serialize, benchmark_deserialize);
criterion_main!(benches);
