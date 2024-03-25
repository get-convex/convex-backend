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

impl<'a, 'b: 'a, RT: Runtime, E: IsolateEnvironment<RT>> ExecutionScope<'a, 'b, RT, E> {
    #[convex_macro::v8_op]
    pub fn op_console_message(
        &mut self,
        level: String,
        messages: Vec<String>,
    ) -> anyhow::Result<()> {
        let state = self.state_mut()?;
        state.environment.trace(level.parse()?, messages)?;
        Ok(())
    }

    #[convex_macro::v8_op]
    pub fn op_console_trace(
        &mut self,
        mut messages: Vec<String>,
        frame_data: Vec<FrameData>,
    ) -> anyhow::Result<()> {
        let js_error = JsError::from_frames("".to_string(), frame_data, None, |s| {
            self.lookup_source_map(s)
        })?;
        let state = self.state_mut()?;
        messages.push(js_error.to_string());
        state.environment.trace(LogLevel::Log, messages)?;
        Ok(())
    }

    #[convex_macro::v8_op]
    pub fn op_console_timeStart(&mut self, label: String) -> anyhow::Result<()> {
        let state = self.state_mut()?;
        if state.console_timers.contains_key(&label) {
            state.environment.trace(
                LogLevel::Warn,
                vec![format!("Timer '{label}' already exists")],
            )?;
        } else {
            state
                .console_timers
                .insert(label, state.unix_timestamp_non_deterministic());
        };
        Ok(())
    }

    #[convex_macro::v8_op]
    pub fn op_console_timeLog(
        &mut self,
        label: String,
        extra_messages: Vec<String>,
    ) -> anyhow::Result<()> {
        let state = self.state_mut()?;
        match state.console_timers.get(&label) {
            None => {
                state.environment.trace(
                    LogLevel::Warn,
                    vec![format!("Timer '{label}' does not exist")],
                )?;
            },
            Some(time) => {
                let now = state
                    .unix_timestamp_non_deterministic()
                    .as_ms_since_epoch()?;
                let duration = now - time.as_ms_since_epoch()?;
                let mut messages = vec![format!("{label}: {duration}ms")];
                messages.extend(extra_messages.into_iter());
                state.environment.trace(LogLevel::Info, messages)?;
            },
        };
        Ok(())
    }

    #[convex_macro::v8_op]
    pub fn op_console_timeEnd(&mut self, label: String) -> anyhow::Result<()> {
        let state = self.state_mut()?;
        match state.console_timers.remove(&label) {
            None => {
                state.environment.trace(
                    LogLevel::Warn,
                    vec![format!("Timer '{label}' does not exist")],
                )?;
            },
            Some(time) => {
                let now = state
                    .unix_timestamp_non_deterministic()
                    .as_ms_since_epoch()?;
                let duration = now - time.as_ms_since_epoch()?;
                state
                    .environment
                    .trace(LogLevel::Info, vec![format!("{label}: {duration}ms")])?;
            },
        };
        Ok(())
    }
}
