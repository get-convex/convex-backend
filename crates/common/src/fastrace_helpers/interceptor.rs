use fastrace::prelude::SpanContext;
use tonic::service::Interceptor;

use crate::http::TRACEPARENT_HEADER_STR;

/// An interceptor that injects the `traceparent` header so that the called
/// service can continue tracing.
#[derive(Copy, Clone, Debug)]
pub struct TraceparentPopulatingInterceptor;

impl Interceptor for TraceparentPopulatingInterceptor {
    fn call(
        &mut self,
        mut request: tonic::Request<()>,
    ) -> Result<tonic::Request<()>, tonic::Status> {
        if let Some(ctx) = SpanContext::current_local_parent() {
            if let Ok(value) = ctx.encode_w3c_traceparent().try_into() {
                request.metadata_mut().insert(TRACEPARENT_HEADER_STR, value);
            }
        }
        Ok(request)
    }
}
