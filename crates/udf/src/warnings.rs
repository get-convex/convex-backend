use std::time::Duration;

use common::{
    knobs::{
        FUNCTION_LIMIT_WARNING_RATIO,
        MAX_SCHEDULED_JOB_ARGUMENT_SIZE_BYTES,
    },
    log_lines::{
        LogLevel,
        LogLine,
        SystemLogMetadata,
    },
    runtime::UnixTimestamp,
};
use sync_types::CanonicalizedUdfPath;

use crate::metrics::log_function_limit_warning;

pub struct SystemWarning {
    pub level: LogLevel,
    pub messages: Vec<String>,
    pub system_log_metadata: SystemLogMetadata,
}

impl SystemWarning {
    pub fn into_log_line(self, timestamp: UnixTimestamp) -> LogLine {
        LogLine::new_system_log_line(
            self.level,
            self.messages,
            timestamp,
            self.system_log_metadata,
        )
    }
}

pub fn approaching_limit_warning(
    actual: usize,
    limit: usize,
    short_code: &'static str,
    get_message: impl Fn() -> String,
    message_suffix: Option<&str>,
    unit: Option<&str>,
    system_udf_path: Option<&CanonicalizedUdfPath>,
) -> Option<SystemWarning> {
    let warning_limit = (*FUNCTION_LIMIT_WARNING_RATIO * (limit as f64)) as usize;
    let should_warn = actual > warning_limit && actual <= limit;
    if !should_warn {
        return None;
    }
    let warning = create_warning(
        actual,
        limit,
        short_code,
        get_message,
        message_suffix,
        unit,
        system_udf_path,
    );
    Some(warning)
}

fn create_warning(
    actual: usize,
    limit: usize,
    short_code: &'static str,
    get_message: impl Fn() -> String,
    message_suffix: Option<&str>,
    unit: Option<&str>,
    system_udf_path: Option<&CanonicalizedUdfPath>,
) -> SystemWarning {
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

    SystemWarning {
        level: LogLevel::Warn,
        messages: vec![full_message],
        system_log_metadata: SystemLogMetadata {
            code: format!("warning:{short_code}"),
        },
    }
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

pub fn scheduled_arg_size_warning(
    args_size: usize,
    system_udf_path: &Option<CanonicalizedUdfPath>,
) -> Option<SystemWarning> {
    // warn even if above the limit, as we aren't enforcing this limit yet
    // TODO: enforce the limit & switch to the usual `approaching_limit_warning`
    let warning_limit =
        (*FUNCTION_LIMIT_WARNING_RATIO * (*MAX_SCHEDULED_JOB_ARGUMENT_SIZE_BYTES as f64)) as usize;
    let should_warn = args_size > warning_limit;
    if !should_warn {
        return None;
    }
    let warning = create_warning(
        args_size,
        *MAX_SCHEDULED_JOB_ARGUMENT_SIZE_BYTES,
        "ScheduledFunctionsArgumentsTooLarge",
        || {
            if args_size > *MAX_SCHEDULED_JOB_ARGUMENT_SIZE_BYTES {
                "Large arguments for a single scheduled function from this mutation. This will \
                 become a hard error in the future"
            } else {
                "Large arguments for a single scheduled function from this mutation"
            }
            .to_string()
        },
        None,
        Some(" bytes"),
        system_udf_path.as_ref(),
    );
    Some(warning)
}
