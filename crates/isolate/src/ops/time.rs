use std::time::Duration;

use deno_core::{
    serde_v8,
    v8::{
        self,
    },
};
use serde_json::value::Number as JsonNumber;

use super::OpProvider;
use crate::environment::AsyncOpRequest;

pub fn async_op_sleep<'b, P: OpProvider<'b>>(
    provider: &mut P,
    args: v8::FunctionCallbackArguments,
    resolver: v8::Global<v8::PromiseResolver>,
) -> anyhow::Result<()> {
    // NOTE: name is only used for error messages.
    let name: String = serde_v8::from_v8(provider.scope(), args.get(1))?;
    let mut millis: f64 = serde_v8::from_v8(provider.scope(), args.get(2))?;
    if millis < 0.0 {
        millis = 0.0;
    }
    let duration = Duration::from_millis(millis as u64);

    let until = provider.unix_timestamp()? + duration;
    provider.start_async_op(AsyncOpRequest::Sleep { name, until }, resolver)
}

#[convex_macro::v8_op]
pub fn op_now<'b, P: OpProvider<'b>>(provider: &mut P) -> anyhow::Result<JsonNumber> {
    // NB: Date.now returns the current Unix timestamp in *milliseconds*. We round
    // to the nearest millisecond to match browsers. Browsers generally don't
    // provide sub-millisecond precision to protect against timing attacks:
    // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Date/now#reduced_time_precision
    let ms_since_epoch: u64 = provider.unix_timestamp()?.as_ms_since_epoch()?;
    let n = JsonNumber::from(ms_since_epoch);
    Ok(n)
}
