use anyhow::Context;
use common::{
    errors::{
        FrameData,
        JsError,
    },
    log_lines::LogLevel,
};

use super::{
    metrics,
    OpProvider,
};

#[convex_macro::v8_op]
pub fn op_console_message<'b, P: OpProvider<'b>>(
    provider: &mut P,
    level: String,
    messages: Vec<String>,
) -> anyhow::Result<()> {
    for message in messages.iter() {
        metrics::log_log_line(message);
    }
    provider.trace(level.parse()?, messages)?;
    Ok(())
}

#[convex_macro::v8_op]
pub fn op_console_trace<'b, P: OpProvider<'b>>(
    provider: &mut P,
    mut messages: Vec<String>,
    frame_data: Vec<FrameData>,
) -> anyhow::Result<()> {
    let js_error = JsError::from_frames("".to_string(), frame_data, None, |s| {
        provider.lookup_source_map(s)
    });
    messages.push(js_error.to_string());
    provider.trace(LogLevel::Log, messages)?;
    Ok(())
}

#[convex_macro::v8_op]
pub fn op_console_time_start<'b, P: OpProvider<'b>>(
    provider: &mut P,
    label: String,
) -> anyhow::Result<()> {
    if provider.console_timers()?.contains_key(&label) {
        provider.trace(
            LogLevel::Warn,
            vec![format!("Timer '{label}' already exists")],
        )?;
    } else {
        let timestamp = provider.unix_timestamp_non_deterministic()?;
        provider.console_timers()?.insert(label, timestamp);
    };
    Ok(())
}

#[convex_macro::v8_op]
pub fn op_console_time_log<'b, P: OpProvider<'b>>(
    provider: &mut P,
    label: String,
    extra_messages: Vec<String>,
) -> anyhow::Result<()> {
    let now = provider
        .unix_timestamp_non_deterministic()?
        .as_ms_since_epoch()?;
    match provider.console_timers()?.get(&label) {
        None => {
            provider.trace(
                LogLevel::Warn,
                vec![format!("Timer '{label}' does not exist")],
            )?;
        },
        Some(time) => {
            let duration = now - time.as_ms_since_epoch()?;
            let mut messages = vec![format!("{label}: {duration}ms")];
            messages.extend(extra_messages.into_iter());
            provider.trace(LogLevel::Info, messages)?;
        },
    };
    Ok(())
}

#[convex_macro::v8_op]
pub fn op_console_time_end<'b, P: OpProvider<'b>>(
    provider: &mut P,
    label: String,
) -> anyhow::Result<()> {
    match provider.console_timers()?.remove(&label) {
        None => {
            provider.trace(
                LogLevel::Warn,
                vec![format!("Timer '{label}' does not exist")],
            )?;
        },
        Some(time) => {
            let now = provider
                .unix_timestamp_non_deterministic()?
                .as_ms_since_epoch()?;
            let duration = now - time.as_ms_since_epoch()?;
            provider.trace(LogLevel::Info, vec![format!("{label}: {duration}ms")])?;
        },
    };
    Ok(())
}
