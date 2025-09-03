use std::time::Duration;

use common::{
    knobs::FUNCTION_LIMIT_WARNING_RATIO,
    log_lines::{
        LogLevel,
        SystemLogMetadata,
    },
};
use sync_types::CanonicalizedUdfPath;

use crate::metrics::log_function_limit_warning;

pub struct SystemWarning {
    pub level: LogLevel,
    pub messages: Vec<String>,
    pub system_log_metadata: SystemLogMetadata,
}

pub fn approaching_limit_warning(
    actual: usize,
    limit: usize,
    short_code: &'static str,
    get_message: impl Fn() -> String,
    message_suffix: Option<&str>,
    unit: Option<&str>,
    system_udf_path: Option<&CanonicalizedUdfPath>,
) -> anyhow::Result<Option<SystemWarning>> {
    let warning_limit = (*FUNCTION_LIMIT_WARNING_RATIO * (limit as f64)) as usize;
    let should_warn = actual > warning_limit && actual <= limit;
    if !should_warn {
        return Ok(None);
    }
    log_function_limit_warning(short_code, system_udf_path);

    let message = get_message();
    let full_message = format!(
        "{message} (actual: {actual}{}, limit: {limit}{}).{}",
        unit.unwrap_or(""),
        unit.unwrap_or(""),
        message_suffix
            .map(|suffix| format!(" {suffix}"))
            .unwrap_or("".to_string()),
    );
    let warning = SystemWarning {
        level: LogLevel::Warn,
        messages: vec![full_message],
        system_log_metadata: SystemLogMetadata {
            code: format!("warning:{short_code}"),
        },
    };
    Ok(Some(warning))
}

pub fn approaching_duration_limit_warning(
    actual: Duration,
    limit: Duration,
    short_code: &'static str,
    message: &str,
    system_udf_path: Option<&CanonicalizedUdfPath>,
) -> anyhow::Result<Option<SystemWarning>> {
    let warning_limit = limit.mul_f64(*FUNCTION_LIMIT_WARNING_RATIO);
    let should_warn = actual > warning_limit && actual <= limit;
    if !should_warn {
        return Ok(None);
    }
    log_function_limit_warning(short_code, system_udf_path);
    let warning = SystemWarning {
        level: LogLevel::Warn,
        messages: vec![format!(
            "{message}. (maximum duration: {limit:?}, actual duration: {actual:?})."
        )],
        system_log_metadata: SystemLogMetadata {
            code: format!("warning:{short_code}"),
        },
    };
    Ok(Some(warning))
}
