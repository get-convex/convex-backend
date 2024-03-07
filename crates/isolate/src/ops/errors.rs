use anyhow::Context;
use common::{
    errors::{
        report_error,
        FrameData,
        JsError,
    },
    runtime::Runtime,
};

use crate::{
    environment::{
        IsolateEnvironment,
        UncatchableDeveloperError,
    },
    execution_scope::ExecutionScope,
};

impl<'a, 'b: 'a, RT: Runtime, E: IsolateEnvironment<RT>> ExecutionScope<'a, 'b, RT, E> {
    #[convex_macro::v8_op]
    pub fn op_throwUncatchableDeveloperError(
        &mut self,
        message: String,
        frame_data: Vec<FrameData>,
    ) -> anyhow::Result<()> {
        let js_error = JsError::from_frames(message.clone(), frame_data, None, |s| {
            self.lookup_source_map(s)
        })?;
        report_error(&mut anyhow::anyhow!(format!(
            "UncatchableDeveloperError: {}",
            message
        )));
        anyhow::bail!(UncatchableDeveloperError { js_error })
    }

    /// Do source mapping to find the stack trace for an error.
    /// NOTE if a UDF throws an error, we call this op and then separately do
    /// source mapping again so the yielded error has structured frame data.
    #[convex_macro::v8_op]
    pub fn op_error_stack(&mut self, frame_data: Vec<FrameData>) -> anyhow::Result<String> {
        let js_error = JsError::from_frames(String::new(), frame_data, None, |s| {
            self.lookup_source_map(s)
        })?;
        Ok(js_error
            .frames
            .expect("JsError::from_frames has frames=None")
            .to_string())
    }
}
