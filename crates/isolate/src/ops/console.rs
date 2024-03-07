use anyhow::Context;
use common::{
    errors::{
        FrameData,
        JsError,
    },
    log_lines::LogLevel,
    runtime::Runtime,
};

use crate::{
    environment::IsolateEnvironment,
    execution_scope::ExecutionScope,
};

fn format_message(level: LogLevel, message: String) -> String {
    format!("[{level}] {message}")
}

impl<'a, 'b: 'a, RT: Runtime, E: IsolateEnvironment<RT>> ExecutionScope<'a, 'b, RT, E> {
    #[convex_macro::v8_op]
    pub fn op_console_message(&mut self, level: String, message: String) -> anyhow::Result<()> {
        let state = self.state_mut();
        state
            .environment
            .trace(format_message(level.parse()?, message))?;
        Ok(())
    }

    #[convex_macro::v8_op]
    pub fn op_console_trace(
        &mut self,
        message: String,
        frame_data: Vec<FrameData>,
    ) -> anyhow::Result<()> {
        let js_error = JsError::from_frames(
            format_message(LogLevel::Log, message),
            frame_data,
            None,
            |s| self.lookup_source_map(s),
        )?;
        let state = self.state_mut();
        state.environment.trace(js_error.to_string())?;
        Ok(())
    }

    #[convex_macro::v8_op]
    pub fn op_console_timeStart(&mut self, label: String) -> anyhow::Result<()> {
        let state = self.state_mut();
        if state.console_timers.contains_key(&label) {
            state.environment.trace(format_message(
                LogLevel::Warn,
                format!("Timer '{label}' already exists"),
            ))?;
        } else {
            state
                .console_timers
                .insert(label, state.unix_timestamp_non_deterministic());
        };
        Ok(())
    }

    #[convex_macro::v8_op]
    pub fn op_console_timeLog(&mut self, label: String, message: String) -> anyhow::Result<()> {
        let state = self.state_mut();
        match state.console_timers.get(&label) {
            None => {
                state.environment.trace(format_message(
                    LogLevel::Warn,
                    format!("Timer '{label}' does not exist"),
                ))?;
            },
            Some(time) => {
                let now = state
                    .unix_timestamp_non_deterministic()
                    .as_ms_since_epoch()?;
                let duration = now - time.as_ms_since_epoch()?;
                let log_line = if message.is_empty() {
                    format!("{label}: {duration}ms")
                } else {
                    format!("{label}: {duration}ms {message}")
                };
                state
                    .environment
                    .trace(format_message(LogLevel::Info, log_line))?;
            },
        };
        Ok(())
    }

    #[convex_macro::v8_op]
    pub fn op_console_timeEnd(&mut self, label: String) -> anyhow::Result<()> {
        let state = self.state_mut();
        match state.console_timers.remove(&label) {
            None => {
                state.environment.trace(format_message(
                    LogLevel::Warn,
                    format!("Timer '{label}' does not exist"),
                ))?;
            },
            Some(time) => {
                let now = state
                    .unix_timestamp_non_deterministic()
                    .as_ms_since_epoch()?;
                let duration = now - time.as_ms_since_epoch()?;
                state.environment.trace(format_message(
                    LogLevel::Info,
                    format!("{label}: {duration}ms"),
                ))?;
            },
        };
        Ok(())
    }
}
