use criterion::{
    criterion_group,
    criterion_main,
    Criterion,
};
use serde_json::json;
use value::ConvexValue;

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
    c.bench_function("to_json::simple", |b| {
        b.iter(|| serde_json::to_string(&serde_json::Value::from(value.clone())))
    });
    let value = ConvexValue::try_from(nested_value()).unwrap();
    c.bench_function("to_json::nested", |b| {
        b.iter(|| serde_json::to_string(&serde_json::Value::from(value.clone())))
    });
}

pub fn benchmark_deserialize(c: &mut Criterion) {
    let value = serde_json::to_string(&simple_value()).unwrap();
    c.bench_function("from_json::simple", |b| {
        b.iter(|| {
            ConvexValue::try_from(serde_json::from_str::<serde_json::Value>(&value).unwrap())
                .unwrap()
        })
    });
    let value = serde_json::to_string(&nested_value()).unwrap();
    c.bench_function("from_json::nested", |b| {
        b.iter(|| {
            ConvexValue::try_from(serde_json::from_str::<serde_json::Value>(&value).unwrap())
                .unwrap()
        })
    });
}

criterion_group!(benches, benchmark_serialize, benchmark_deserialize);
criterion_main!(benches);
