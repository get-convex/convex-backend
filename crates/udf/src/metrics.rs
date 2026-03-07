use metrics::{
    log_counter_with_labels,
    register_convex_counter,
    StaticMetricLabel,
};
use sync_types::CanonicalizedUdfPath;

register_convex_counter!(
    FUNCTION_LIMIT_WARNING_TOTAL,
    "Count of functions that exceeded some limit warning level",
    &["limit", "system_udf_path"]
);
pub fn log_function_limit_warning(
    limit_name: &'static str,
    system_udf_path: Option<&CanonicalizedUdfPath>,
) {
    let labels = match system_udf_path {
        Some(udf_path) => vec![
            StaticMetricLabel::new("limit", limit_name),
            StaticMetricLabel::new("system_udf_path", udf_path.to_string()),
        ],
        None => vec![
            StaticMetricLabel::new("limit", limit_name),
            StaticMetricLabel::new("system_udf_path", "none"),
        ],
    };
    log_counter_with_labels(&FUNCTION_LIMIT_WARNING_TOTAL, 1, labels);
}
