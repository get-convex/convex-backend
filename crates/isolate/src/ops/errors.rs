use anyhow::Context;
use common::errors::{
    report_error_sync,
    FrameData,
    JsError,
};
use deno_core::v8::{
    self,
    scope,
};

use super::OpProvider;
use crate::{
    environment::UncatchableDeveloperError,
    strings,
};

pub(crate) fn throw_uncatchable_developer_error<'b, P: OpProvider<'b>>(
    provider: &mut P,
    message: String,
) -> anyhow::Result<!> {
    let frame_data: anyhow::Result<Vec<FrameData>> = try {
        let mut scope = provider.scope();
        scope!(let scope, &mut scope);
        let empty_string = strings::empty.create(scope)?;
        let error = v8::Exception::error(scope, empty_string).try_cast::<v8::Object>()?;
        let stack_string = strings::stack.create(scope)?;
        // This calls `prepareStackTrace` that populates `__frameData`
        error
            .get(scope, stack_string.into())
            .context("Error.stack threw")?;
        let frame_data_str = strings::__frameData.create(scope)?;
        let frame_data_json = error
            .get(scope, frame_data_str.into())
            .context("Error.__frameData threw")?
            .try_cast::<v8::String>()?;
        let frame_data_json = frame_data_json.to_rust_string_lossy(scope);
        serde_json::from_str(&frame_data_json)?
    };
    let js_error = JsError::from_frames(
        message.clone(),
        match frame_data {
            Ok(data) => data,
            Err(mut e) => {
                report_error_sync(&mut e);
                vec![]
            },
        },
        None,
        |s| provider.lookup_source_map(s),
    );
    report_error_sync(&mut anyhow::anyhow!(format!(
        "UncatchableDeveloperError: {}",
        message
    )));
    anyhow::bail!(UncatchableDeveloperError { js_error })
}

#[convex_macro::v8_op]
pub fn op_throw_uncatchable_developer_error<'b, P: OpProvider<'b>>(
    provider: &mut P,
    message: String,
) -> anyhow::Result<()> {
    throw_uncatchable_developer_error(provider, message)?;
}

/// Do source mapping to find the stack trace for an error.
/// NOTE if a UDF throws an error, we call this op and then separately do
/// source mapping again so the yielded error has structured frame data.
#[convex_macro::v8_op]
pub fn op_error_stack<'b, P: OpProvider<'b>>(
    provider: &mut P,
    frame_data: Vec<FrameData>,
) -> anyhow::Result<String> {
    let js_error = JsError::from_frames(String::new(), frame_data, None, |s| {
        provider.lookup_source_map(s)
    });
    Ok(js_error
        .frames
        .expect("JsError::from_frames has frames=None")
        .to_string())
}
