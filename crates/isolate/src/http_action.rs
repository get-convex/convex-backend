use core::fmt;

use common::{
    http::HttpResponse,
    types::HttpActionRoute,
};
use futures::stream::BoxStream;
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
    pub fn route_for_failure(&self) -> anyhow::Result<HttpActionRoute> {
        let path = self.url.path();
        Ok(HttpActionRoute {
            method: self.method.to_string().parse()?,
            path: path.to_string(),
        })
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpActionResponse {
    pub body: Option<Vec<u8>>,
    status: StatusCode,
    pub headers: HeaderMap,
}

impl HttpActionResponse {
    pub fn from_http_response(response: HttpResponse) -> Self {
        Self {
            body: response.body,
            status: response.status,
            headers: response.headers,
        }
    }

    pub fn from_text(status: StatusCode, body: String) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref()),
        );
        HttpActionResponse {
            body: Some(body.into_bytes()),
            status,
            headers,
        }
    }

    pub fn from_json(status: StatusCode, body: JsonValue) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static(mime::APPLICATION_JSON.as_ref()),
        );
        HttpActionResponse {
            body: Some(body.to_string().into_bytes()),
            status,
            headers,
        }
    }

    pub fn body(&self) -> &Option<Vec<u8>> {
        &self.body
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
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
