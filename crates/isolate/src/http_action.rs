use core::fmt;

use bytes::Bytes;
use common::types::{
    HttpActionRoute,
    RoutableMethod,
};
use futures::{
    channel::mpsc,
    stream::BoxStream,
};
use headers::{
    HeaderMap,
    HeaderValue,
};
use http::{
    header::CONTENT_TYPE,
    Method,
    StatusCode,
};
use serde_json::Value as JsonValue;
use url::Url;

pub const HTTP_ACTION_BODY_LIMIT: usize = 20 << 20;

pub struct HttpActionRequest {
    pub head: HttpActionRequestHead,
    pub body: Option<BoxStream<'static, anyhow::Result<bytes::Bytes>>>,
}

impl fmt::Debug for HttpActionRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpActionRequest")
            .field("head", &self.head)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpActionRequestHead {
    pub headers: HeaderMap,
    pub url: Url,
    pub method: Method,
}

impl HttpActionRequestHead {
    // HttpActionRoutes should normally come from the router, but in cases where
    // we fail to do so, we use this to construct a route we can use for the
    // purposes of logging
    pub fn route_for_failure(&self) -> HttpActionRoute {
        let path = self.url.path();
        HttpActionRoute {
            // TODO: we want this to be infallible so we can always log something, so pick `Get`
            // if the method doesn't parse. The better solution is to have a separate struct for
            // logging that allows `method` to be any string.
            method: self
                .method
                .to_string()
                .parse()
                .unwrap_or(RoutableMethod::Get),
            path: path.to_string(),
        }
    }
}

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for HttpActionRequest {
    type Parameters = ();

    type Strategy = impl proptest::strategy::Strategy<Value = HttpActionRequest>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        use futures::{
            stream,
            StreamExt,
        };
        use proptest::prelude::*;
        use proptest_http::{
            ArbitraryHeaderMap,
            ArbitraryMethod,
            ArbitraryUri,
        };
        prop_compose! {
            fn inner()(
                ArbitraryHeaderMap(headers) in any::<ArbitraryHeaderMap>(),
                ArbitraryMethod(method) in any::<ArbitraryMethod>(),
                ArbitraryUri(uri) in any::<ArbitraryUri>(),
                body in any::<Option<Vec<u8>>>()) -> anyhow::Result<HttpActionRequest> {
                    let origin: String = "http://example-deployment.convex.site/".to_string();
                    let path_and_query: String =  uri.path_and_query().ok_or_else(|| anyhow::anyhow!("No path and query"))?.to_string();
                    let url: Url = Url::parse(&(origin + &path_and_query))?;
                Ok(HttpActionRequest {
                    head: HttpActionRequestHead {
                        headers,
                        method,
                        url,
                    },
                    body: body.map(|body| stream::once(async move { Ok(body.into())}).boxed())

                })
            }
        };
        inner().prop_filter_map("Invalid HttpActionRequest", |r| r.ok())
    }
}

/// HTTP Action responses are usually streamed via HttpActionResponsePart, so
/// this struct is only used in tests for convenience
#[cfg(any(test, feature = "testing"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpActionResponse {
    pub body: Option<Vec<u8>>,
    pub status: StatusCode,
    pub headers: HeaderMap,
}

#[cfg(any(test, feature = "testing"))]
impl HttpActionResponse {
    pub fn body(&self) -> &Option<Vec<u8>> {
        &self.body
    }
}

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for HttpActionResponse {
    type Parameters = ();

    type Strategy = impl proptest::strategy::Strategy<Value = HttpActionResponse>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        use proptest_http::{
            ArbitraryHeaderMap,
            ArbitraryStatusCode,
        };
        prop_compose! {
            fn inner()(
                ArbitraryHeaderMap(headers) in any::<ArbitraryHeaderMap>(),
                ArbitraryStatusCode(status) in any::<ArbitraryStatusCode>(),
                body in any::<Option<Vec<u8>>>()) -> anyhow::Result<HttpActionResponse> {
                Ok(HttpActionResponse {
                    status,
                    headers,
                    body

                })
            }
        };
        inner().prop_filter_map("Invalid HttpActionRequest", |r| r.ok())
    }
}

#[derive(Debug, Clone)]
pub enum HttpActionResponsePart {
    Head(HttpActionResponseHead),
    BodyChunk(Bytes),
}

impl HttpActionResponsePart {
    pub fn from_text(status: StatusCode, message: String) -> Vec<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref()),
        );
        let head = Self::Head(HttpActionResponseHead { status, headers });
        let body = Self::BodyChunk(message.into_bytes().into());
        vec![head, body]
    }

    pub fn from_json(status: StatusCode, body: JsonValue) -> Vec<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static(mime::APPLICATION_JSON.as_ref()),
        );
        let head = Self::Head(HttpActionResponseHead { status, headers });
        let body_chunk = HttpActionResponsePart::BodyChunk(body.to_string().into_bytes().into());
        vec![head, body_chunk]
    }
}

#[derive(Debug, Clone)]
pub struct HttpActionResponseHead {
    pub status: StatusCode,
    pub headers: HeaderMap,
}

#[derive(Debug, Clone)]
pub struct HttpActionResponseStreamer {
    head: Option<HttpActionResponseHead>,
    total_bytes_sent: usize,
    pub sender: mpsc::UnboundedSender<HttpActionResponsePart>,
}

impl HttpActionResponseStreamer {
    pub fn new(sender: mpsc::UnboundedSender<HttpActionResponsePart>) -> Self {
        Self {
            head: None,
            total_bytes_sent: 0,
            sender,
        }
    }

    pub fn has_started(&self) -> bool {
        self.head.is_some()
    }

    pub fn head(&self) -> Option<&HttpActionResponseHead> {
        self.head.as_ref()
    }

    pub fn total_bytes_sent(&self) -> usize {
        self.total_bytes_sent
    }

    fn send_head(&mut self, head: HttpActionResponseHead) -> anyhow::Result<()> {
        if self.has_started() {
            anyhow::bail!("Sending HTTP response head after other response parts");
        };
        self.head = Some(head.clone());
        self.sender
            .unbounded_send(HttpActionResponsePart::Head(head))?;
        Ok(())
    }

    fn send_body(&mut self, bytes: Bytes) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.has_started(),
            "Sending response body before response head"
        );
        self.total_bytes_sent += bytes.len();
        self.sender
            .unbounded_send(HttpActionResponsePart::BodyChunk(bytes))?;
        Ok(())
    }

    pub fn send_part(&mut self, part: HttpActionResponsePart) -> anyhow::Result<()> {
        match part {
            HttpActionResponsePart::Head(h) => self.send_head(h)?,
            HttpActionResponsePart::BodyChunk(b) => self.send_body(b)?,
        }
        Ok(())
    }
}
