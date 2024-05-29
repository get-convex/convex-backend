use metrics::{
    log_counter_with_labels,
    register_convex_counter,
    IntoLabel,
    MetricLabel,
};

use crate::ClientPurpose;

register_convex_counter!(
    HTTP_CLIENT_REQUESTS_TOTAL,
    "Count of requests made using the internal cached HTTP client",
    &["purpose", "cache_hit"]
);

pub fn log_http_response(purpose: ClientPurpose, cache_hit: bool) {
    log_counter_with_labels(
        &HTTP_CLIENT_REQUESTS_TOTAL,
        1,
        vec![
            MetricLabel::new_const("purpose", purpose.into()),
            MetricLabel::new_const("cache_hit", cache_hit.as_label()),
        ],
    )
}
