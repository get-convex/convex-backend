use std::{
    pin::Pin,
    sync::Arc,
    task::{
        Context,
        Poll,
    },
};

use bytes::Bytes;
use http::Request;
use hyper::body::{
    Body,
    Frame,
    SizeHint,
};
use metrics::{
    log_counter_with_labels,
    register_convex_counter,
    register_convex_histogram,
    StaticMetricLabel,
    StatusTimer,
};
use pin_project::pin_project;
use tokio_metrics::Instrumented;
use tonic::async_trait;
use tonic_middleware::{
    Middleware,
    ServiceBound,
};
use tower::Service;

use crate::{
    grpc::KnownMethods,
    runtime::TaskManager,
};

#[derive(Clone)]
pub(crate) struct TokioInstrumentationService<S> {
    pub(crate) known_methods: Arc<KnownMethods>,
    pub(crate) inner: S,
}

impl<S> TokioInstrumentationService<S> {
    pub(crate) fn new(known_methods: Arc<KnownMethods>, inner: S) -> Self {
        Self {
            known_methods,
            inner,
        }
    }
}

impl<S, T> Service<Request<T>> for TokioInstrumentationService<S>
where
    S: Service<Request<T>>,
{
    type Error = S::Error;
    type Future = Instrumented<S::Future>;
    type Response = S::Response;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<T>) -> Self::Future {
        let name = self
            .known_methods
            .get(req.uri().path())
            .copied()
            .unwrap_or("grpc_handler");
        TaskManager::instrument(name, self.inner.call(req))
    }
}

register_convex_counter!(
    GRPC_SERVER_STARTED_TOTAL,
    "Total number of RPCs started by the server. This minus the total number of RPCs handled \
     (from the histogram) will give you the number of RPCs that are in flight",
    &["method"]
);

register_convex_histogram!(
    GRPC_HANDLE_DURATION_SECONDS,
    "RPC call duration",
    &["status", "method", "grpc_status"]
);

#[derive(Clone)]
pub struct LoggingMiddleware {
    known_methods: Arc<KnownMethods>,
}

impl LoggingMiddleware {
    pub fn new(known_methods: Arc<KnownMethods>) -> Self {
        Self { known_methods }
    }
}

#[async_trait]
impl<S> Middleware<S> for LoggingMiddleware
where
    S: ServiceBound,
    S::Future: Send,
{
    async fn call(
        &self,
        req: http::Request<tonic::body::Body>,
        mut service: S,
    ) -> Result<http::Response<tonic::body::Body>, S::Error> {
        let method = self
            .known_methods
            .get(req.uri().path())
            .copied()
            .unwrap_or("unknown");
        log_counter_with_labels(
            &GRPC_SERVER_STARTED_TOTAL,
            1,
            vec![StaticMetricLabel::new("method", method)],
        );
        let mut timer = StatusTimer::new(&GRPC_HANDLE_DURATION_SECONDS);
        timer.add_label(StaticMetricLabel::new("method", method));
        // We don't set this to "Unknown" because that's a real gRPC status, whereas we
        // may never know a gRPC status if an error occurs.
        timer.add_label(StaticMetricLabel::new("grpc_status", ""));
        let mut response_logger = ResponseLogger {
            method,
            timer: Some(timer),
            msg_count: 0,
            size: 0,
            grpc_status: None,
        };
        let response = service.call(req).await?;
        // `grpc-status` may be in the headers for unary responses or if an error occurs
        // immediately.
        if let Some(status) = response
            .headers()
            .get("grpc-status")
            .map(|v| format!("{:?}", tonic::Code::from_bytes(v.as_bytes())))
        {
            response_logger.grpc_status = Some(status);
        }
        let (parts, body) = response.into_parts();
        let logging_body = LoggingBody::new(body, response_logger);
        let wrapped_body = tonic::body::Body::new(logging_body);
        Ok(tonic::codegen::http::Response::from_parts(
            parts,
            wrapped_body,
        ))
    }
}

struct ResponseLogger {
    pub method: &'static str,
    pub msg_count: usize,
    pub size: usize,
    pub grpc_status: Option<String>,
    pub timer: Option<StatusTimer>,
}

#[pin_project]
struct LoggingBody<B> {
    #[pin]
    inner: B,
    logger: ResponseLogger,
}

impl<B> LoggingBody<B> {
    pub fn new(inner: B, logger: ResponseLogger) -> Self {
        Self { inner, logger }
    }
}

impl<B> Body for LoggingBody<B>
where
    B: Body<Data = Bytes> + Send + 'static,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>> + Send + Sync,
{
    type Data = Bytes;
    type Error = B::Error;

    /// Intercept every HTTP/2 frame
    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.project();
        match this.inner.poll_frame(cx) {
            Poll::Ready(Some(Ok(frame))) => {
                // Data frame
                if let Some(buf) = frame.data_ref() {
                    this.logger.msg_count += 1;
                    this.logger.size += buf.len();
                }
                // Trailers frame (EOS)
                else if let Some(trailers) = frame.trailers_ref()
                    && let Some(status) = trailers
                        .get("grpc-status")
                        .map(|v| format!("{:?}", tonic::Code::from_bytes(v.as_bytes())))
                {
                    this.logger.grpc_status = Some(status);
                }
                // return the frame unchanged
                Poll::Ready(Some(Ok(frame)))
            },

            other => other, // pass through errors or end-of-stream
        }
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}

impl Drop for ResponseLogger {
    fn drop(&mut self) {
        // If the status is set, this is still a "success" at the network
        // level even if the gRPC status is an error, so we complete the
        // timer with a success status.
        let duration;
        let mut timer = self.timer.take().expect("Someone else took the timer?");
        if let Some(grpc_status) = &self.grpc_status {
            timer.replace_label(
                StaticMetricLabel::new("grpc_status", ""),
                StaticMetricLabel::new("grpc_status", grpc_status.clone()),
            );
            duration = timer.finish();
        } else {
            // The timer will drop here with status "Error", as we didn't call `finish()`
            duration = timer.elapsed();
        };
        tracing::debug!(
            target: "convex-grpc",
            method      = %self.method,
            grpc_status = %self.grpc_status.take().as_deref().unwrap_or("<unspecified>"),
            resp_msgs    = self.msg_count,
            resp_bytes   = self.size,
            duration_ms = %format!("{:.3}", duration.as_secs_f64() * 1000.0),
        );
    }
}
