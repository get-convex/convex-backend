use metrics::{
    log_counter_with_tags,
    metric_tag_const_value,
    register_convex_counter,
};

register_convex_counter!(
    VALUE_SERIALIZATION_TOTAL,
    "Count of value de/serializations",
    &["action", "type"]
);

pub fn log_serialized_set() {
    log_counter_with_tags(
        &VALUE_SERIALIZATION_TOTAL,
        1,
        vec![
            metric_tag_const_value("action", "serialize"),
            metric_tag_const_value("type", "set"),
        ],
    );
}

pub fn log_serialized_map() {
    log_counter_with_tags(
        &VALUE_SERIALIZATION_TOTAL,
        1,
        vec![
            metric_tag_const_value("action", "serialize"),
            metric_tag_const_value("type", "map"),
        ],
    );
}

pub fn log_deserialized_set() {
    log_counter_with_tags(
        &VALUE_SERIALIZATION_TOTAL,
        1,
        vec![
            metric_tag_const_value("action", "deserialize"),
            metric_tag_const_value("type", "set"),
        ],
    );
}

pub fn log_deserialized_map() {
    log_counter_with_tags(
        &VALUE_SERIALIZATION_TOTAL,
        1,
        vec![
            metric_tag_const_value("action", "serialize"),
            metric_tag_const_value("type", "map"),
        ],
    );
}
