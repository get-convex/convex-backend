use metrics::{
    log_counter_with_labels,
    register_convex_counter,
    StaticMetricLabel,
};

register_convex_counter!(
    VALUE_SERIALIZATION_TOTAL,
    "Count of value de/serializations",
    &["action", "type"]
);

pub fn log_serialized_set() {
    log_counter_with_labels(
        &VALUE_SERIALIZATION_TOTAL,
        1,
        vec![
            StaticMetricLabel::new("action", "serialize"),
            StaticMetricLabel::new("type", "set"),
        ],
    );
}

pub fn log_serialized_map() {
    log_counter_with_labels(
        &VALUE_SERIALIZATION_TOTAL,
        1,
        vec![
            StaticMetricLabel::new("action", "serialize"),
            StaticMetricLabel::new("type", "map"),
        ],
    );
}

pub fn log_deserialized_set() {
    log_counter_with_labels(
        &VALUE_SERIALIZATION_TOTAL,
        1,
        vec![
            StaticMetricLabel::new("action", "deserialize"),
            StaticMetricLabel::new("type", "set"),
        ],
    );
}

pub fn log_deserialized_map() {
    log_counter_with_labels(
        &VALUE_SERIALIZATION_TOTAL,
        1,
        vec![
            StaticMetricLabel::new("action", "serialize"),
            StaticMetricLabel::new("type", "map"),
        ],
    );
}
