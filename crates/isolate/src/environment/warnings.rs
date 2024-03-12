use std::time::Duration;

use common::{
    knobs::FUNCTION_LIMIT_WARNING_RATIO,
    log_lines::LogLine,
};
use sync_types::CanonicalizedUdfPath;

use crate::metrics::log_function_limit_warning;

pub fn warning_if_approaching_limit(
    actual: usize,
    limit: usize,
    short_code: &'static str,
    get_message: impl Fn() -> String,
    message_suffix: Option<&str>,
    unit: Option<&str>,
    system_udf_path: Option<&CanonicalizedUdfPath>,
) -> Option<LogLine> {
    let warning_limit = (*FUNCTION_LIMIT_WARNING_RATIO * (limit as f64)) as usize;
    if actual > warning_limit && actual <= limit {
        log_function_limit_warning(short_code, system_udf_path);
        let message = get_message();
        Some(LogLine::Unstructured(format!(
            "[WARN] {message} (actual: {actual}{}, limit: {limit}{}).{}",
            unit.unwrap_or(""),
            unit.unwrap_or(""),
            message_suffix
                .map(|suffix| format!(" {}", suffix))
                .unwrap_or("".to_string())
        )))
    } else {
        None
    }
}

pub fn warning_if_approaching_duration_limit(
    actual: Duration,
    limit: Duration,
    short_code: &'static str,
    message: &str,
    system_udf_path: Option<&CanonicalizedUdfPath>,
) -> Option<LogLine> {
    let warning_limit = limit.mul_f64(*FUNCTION_LIMIT_WARNING_RATIO);
    if actual > warning_limit && actual <= limit {
        log_function_limit_warning(short_code, system_udf_path);
        Some(LogLine::Unstructured(format!(
            "[WARN] {message}. (maximum duration: {limit:?}, actual duration: {actual:?}).",
        )))
    } else {
        None
    }
}
