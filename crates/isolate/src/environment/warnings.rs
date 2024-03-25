use std::time::Duration;

use common::{
    knobs::FUNCTION_LIMIT_WARNING_RATIO,
    log_lines::{
        LogLevel,
        SystemLogMetadata,
    },
    runtime::Runtime,
};
use sync_types::CanonicalizedUdfPath;

use super::IsolateEnvironment;
use crate::metrics::log_function_limit_warning;

pub fn add_warning_if_approaching_limit<RT: Runtime, E: IsolateEnvironment<RT>>(
    environment: &mut E,
    actual: usize,
    limit: usize,
    short_code: &'static str,
    get_message: impl Fn() -> String,
    message_suffix: Option<&str>,
    unit: Option<&str>,
    system_udf_path: Option<&CanonicalizedUdfPath>,
) -> anyhow::Result<()> {
    let warning_limit = (*FUNCTION_LIMIT_WARNING_RATIO * (limit as f64)) as usize;
    let should_warn = actual > warning_limit && actual <= limit;
    if !should_warn {
        return Ok(());
    }
    log_function_limit_warning(short_code, system_udf_path);
    let message = get_message();
    let full_message = format!(
        "{message} (actual: {actual}{}, limit: {limit}{}).{}",
        unit.unwrap_or(""),
        unit.unwrap_or(""),
        message_suffix
            .map(|suffix| format!(" {}", suffix))
            .unwrap_or("".to_string()),
    );
    environment.trace_system(
        LogLevel::Warn,
        vec![full_message],
        SystemLogMetadata {
            code: format!("warning:{short_code}"),
        },
    )
}

pub fn add_warning_if_approaching_duration_limit<RT: Runtime, E: IsolateEnvironment<RT>>(
    environment: &mut E,
    actual: Duration,
    limit: Duration,
    short_code: &'static str,
    message: &str,
    system_udf_path: Option<&CanonicalizedUdfPath>,
) -> anyhow::Result<()> {
    let warning_limit = limit.mul_f64(*FUNCTION_LIMIT_WARNING_RATIO);
    let should_warn = actual > warning_limit && actual <= limit;
    if !should_warn {
        return Ok(());
    }
    log_function_limit_warning(short_code, system_udf_path);
    environment.trace_system(
        LogLevel::Warn,
        vec![format!(
            "{message}. (maximum duration: {limit:?}, actual duration: {actual:?})."
        )],
        SystemLogMetadata {
            code: format!("warning:{short_code}"),
        },
    )
}
