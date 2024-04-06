use std::collections::BTreeMap;

use minitrace::{
    collector::SpanContext,
    Span,
};
use rand::Rng;

use crate::{
    knobs::REQUEST_TRACE_SAMPLE_PERCENT,
    runtime::Runtime,
};

#[derive(Clone)]
pub struct EncodedSpan(pub Option<String>);

impl EncodedSpan {
    pub fn empty() -> Self {
        Self(None)
    }

    /// Encodes the passed in `SpanContext`
    pub fn from_parent(parent: Option<SpanContext>) -> Self {
        Self(parent.map(|ctx| ctx.encode_w3c_traceparent()))
    }
}

/// Given an instance name returns a span with the sample percentage specified
/// in `knobs.rs`
pub fn get_sampled_span<RT: Runtime>(
    request_name: String,
    rt: RT,
    properties: BTreeMap<String, String>,
) -> Span {
    let should_sample = rt
        .clone()
        .with_rng(|rng| rng.gen_bool(*REQUEST_TRACE_SAMPLE_PERCENT));
    match should_sample {
        true => Span::root(request_name, SpanContext::random()).with_properties(|| properties),
        false => Span::noop(),
    }
}

/// Creates a root span from an encoded parent trace
pub fn initialize_root_from_parent(span_name: &'static str, encoded_parent: EncodedSpan) -> Span {
    if let Some(p) = encoded_parent.0 {
        if let Some(ctx) = SpanContext::decode_w3c_traceparent(p.as_str()) {
            return Span::root(span_name, ctx);
        }
    }
    Span::noop()
}
