use fastrace::{
    future::{
        FutureExt,
        InSpan,
    },
    prelude::SpanContext,
    Span,
};
use tower::{
    Layer,
    Service,
};

use crate::http::TRACEPARENT_HEADER;

/// A layer that intercepts the provided `traceparent` HTTP header and starts a
/// new `fastrace` root span.
#[derive(Copy, Clone, Debug)]
pub struct TraceparentReceivingLayer;

impl<S> Layer<S> for TraceparentReceivingLayer {
    type Service = TraceparentReceivingService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TraceparentReceivingService(inner)
    }
}

#[derive(Clone, Debug)]
pub struct TraceparentReceivingService<S>(S);

impl<S, B> Service<http::Request<B>> for TraceparentReceivingService<S>
where
    S: Service<http::Request<B>>,
{
    type Error = S::Error;
    type Future = InSpan<S::Future>;
    type Response = S::Response;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.0.poll_ready(cx)
    }

    fn call(&mut self, req: http::Request<B>) -> Self::Future {
        let span = if let Some(parent) = req.headers().get(TRACEPARENT_HEADER) {
            if let Some(context) = parent
                .to_str()
                .ok()
                .and_then(SpanContext::decode_w3c_traceparent)
            {
                let path = req.uri().path();
                Span::root(path.to_owned(), context).with_property(|| ("span.kind", "server"))
            } else {
                tracing::warn!("invalid traceparent: {:?}", parent);
                Span::noop()
            }
        } else {
            Span::noop()
        };
        self.0.call(req).in_span(span)
    }
}
