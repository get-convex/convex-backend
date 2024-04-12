use metrics::{
    log_counter_with_labels,
    register_convex_counter,
    MetricLabel,
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
            MetricLabel::new("action", "serialize"),
            MetricLabel::new("type", "set"),
        ],
    );
}

pub fn log_serialized_map() {
    log_counter_with_labels(
        &VALUE_SERIALIZATION_TOTAL,
        1,
        vec![
            MetricLabel::new("action", "serialize"),
            MetricLabel::new("type", "map"),
        ],
    );
}

pub fn log_deserialized_set() {
    log_counter_with_labels(
        &VALUE_SERIALIZATION_TOTAL,
        1,
        vec![
            MetricLabel::new("action", "deserialize"),
            MetricLabel::new("type", "set"),
        ],
    );
}

pub fn log_deserialized_map() {
    log_counter_with_labels(
        &VALUE_SERIALIZATION_TOTAL,
        1,
        vec![
            MetricLabel::new("action", "serialize"),
            MetricLabel::new("type", "map"),
        ],
    );
}
