use std::{
    collections::{
        BTreeMap,
        HashMap,
    },
    sync::atomic::{
        AtomicU64,
        Ordering,
    },
};

use async_trait::async_trait;
use futures::{
    future::BoxFuture,
    StreamExt,
    TryStreamExt,
};
use http::StatusCode;
use reqwest::{
    redirect,
    Body,
    Proxy,
    Url,
};

use crate::http::{
    HttpRequestStream,
    HttpResponseStream,
};

/// Http client used for fetch syscall.
#[async_trait]
pub trait FetchClient: Send + Sync {
    async fn fetch(&self, request: HttpRequestStream) -> anyhow::Result<HttpResponseStream>;

    /// Unrestricted, unproxied fetch to be used for internal purposes only.
    /// Customer UDFs should never have access to this method. A `purpose`
    /// parameter is required (but not used) just to make it more obvious why
    /// we're using this method.
    /// E.g. usage tracking can use this to talk directly over the
    /// internal network, bypassing the regular
    /// proxied fetch and its associated security limitations.
    async fn internal_fetch(
        &self,
        request: HttpRequestStream,
        purpose: InternalFetchPurpose,
    ) -> anyhow::Result<HttpResponseStream>;
}

#[derive(Clone)]
pub struct ProxiedFetchClient {
    http_client: reqwest::Client,
    internal_http_client: reqwest::Client,
}

impl ProxiedFetchClient {
    pub fn new(proxy_url: Option<Url>, client_id: String) -> Self {
        let mut builder = reqwest::Client::builder().redirect(redirect::Policy::none());
        // It's okay to panic on these errors, as they indicate a serious programming
        // error -- building the reqwest client is expected to be infallible.
        if let Some(proxy_url) = proxy_url {
            let proxy = Proxy::all(proxy_url)
                .expect("Infallible conversion from URL type to URL type")
                .custom_http_auth(
                    client_id
                        .try_into()
                        .expect("Backend name is not valid ASCII?"),
                );
            builder = builder.proxy(proxy);
        }
        builder = builder.user_agent("Convex/1.0");
        Self {
            http_client: builder.build().expect("Failed to build reqwest client"),
            internal_http_client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl FetchClient for ProxiedFetchClient {
    async fn fetch(&self, request: HttpRequestStream) -> anyhow::Result<HttpResponseStream> {
        let mut request_builder = self
            .http_client
            .request(request.method, request.url.as_str());
        let body = Body::wrap_stream(request.body);
        request_builder = request_builder.body(body);
        for (name, value) in &request.headers {
            request_builder = request_builder.header(name.as_str(), value.as_bytes());
        }
        let raw_request = request_builder.build()?;
        let raw_response = self.http_client.execute(raw_request).await?;
        if raw_response.status() == StatusCode::PROXY_AUTHENTICATION_REQUIRED {
            // SSRF mitigated -- our proxy blocked this request because it was
            // directed at a non-public IP range. Don't send back the raw HTTP response as
            // it leaks internal implementation details in the response headers.
            anyhow::bail!("Request to {} forbidden", request.url);
        }
        let status = raw_response.status();
        let headers = raw_response.headers().to_owned();
        let response = HttpResponseStream {
            status,
            headers,
            url: Some(request.url),
            body: Some(raw_response.bytes_stream().map_err(|e| e.into()).boxed()),
        };
        Ok(response)
    }

    async fn internal_fetch(
        &self,
        request: HttpRequestStream,
        _purpose: InternalFetchPurpose,
    ) -> anyhow::Result<HttpResponseStream> {
        let mut request_builder = self
            .internal_http_client
            .request(request.method, request.url.as_str());
        let body = Body::wrap_stream(request.body);
        request_builder = request_builder.body(body);
        for (name, value) in &request.headers {
            request_builder = request_builder.header(name.as_str(), value.as_bytes());
        }
        let raw_request = request_builder.build()?;
        let raw_response = self.internal_http_client.execute(raw_request).await?;
        let status = raw_response.status();
        let headers = raw_response.headers().to_owned();
        let response = HttpResponseStream {
            status,
            headers,
            url: Some(request.url),
            body: Some(raw_response.bytes_stream().map_err(|e| e.into()).boxed()),
        };
        Ok(response)
    }
}

type HandlerFn = Box<
    dyn Fn(HttpRequestStream) -> BoxFuture<'static, anyhow::Result<HttpResponseStream>>
        + Send
        + Sync
        + 'static,
>;

pub struct StaticFetchClient {
    router: BTreeMap<url::Url, HashMap<http::Method, HandlerFn>>,
    num_calls: AtomicU64,
}

impl StaticFetchClient {
    pub fn new() -> Self {
        Self {
            router: BTreeMap::new(),
            num_calls: AtomicU64::new(0),
        }
    }

    pub fn register_http_route<F>(&mut self, url: url::Url, method: http::Method, handler: F)
    where
        F: Fn(HttpRequestStream) -> BoxFuture<'static, anyhow::Result<HttpResponseStream>>
            + Send
            + Sync
            + 'static,
    {
        self.router
            .entry(url)
            .or_default()
            .insert(method, Box::new(handler));
    }

    /// Returns how many times a fetch client has been called
    pub fn num_calls(&self) -> u64 {
        self.num_calls.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl FetchClient for StaticFetchClient {
    async fn fetch(&self, request: HttpRequestStream) -> anyhow::Result<HttpResponseStream> {
        self.num_calls.fetch_add(1, Ordering::Relaxed);
        let handler = self
            .router
            .get(&request.url)
            .and_then(|methods| methods.get(&request.method))
            .unwrap_or_else(|| {
                panic!(
                    "could not find route {} with method {}",
                    request.url, request.method
                )
            });
        handler(request).await
    }

    async fn internal_fetch(
        &self,
        request: HttpRequestStream,
        _purpose: InternalFetchPurpose,
    ) -> anyhow::Result<HttpResponseStream> {
        self.fetch(request).await
    }
}

pub enum InternalFetchPurpose {
    AccessTokenAuth,
}

#[cfg(test)]
mod tests {
    use errors::ErrorMetadataAnyhowExt;
    use futures::FutureExt;
    use http::{
        HeaderMap,
        Method,
        StatusCode,
    };

    use super::ProxiedFetchClient;
    use crate::http::{
        categorize_http_response_stream,
        fetch::{
            FetchClient,
            StaticFetchClient,
        },
        HttpRequest,
        HttpRequestStream,
        HttpResponse,
        HttpResponseStream,
        CONVEX_CLIENT_HEADER,
        CONVEX_CLIENT_HEADER_VALUE,
    };

    #[tokio::test]
    async fn test_fetch_bad_url() -> anyhow::Result<()> {
        let client = ProxiedFetchClient::new(None, "".to_owned());
        let request = HttpRequest {
            headers: Default::default(),
            url: "http://\"".parse()?,
            method: Method::GET,
            body: None,
        };
        let Err(err) = client.fetch(request.into()).await else {
            panic!("Expected Invalid URL error");
        };

        // Ensure it doesn't panic. Regression test for.
        // https://github.com/seanmonstar/reqwest/issues/668
        assert!(err.to_string().contains("Parsed Url is not a valid Uri"));

        Ok(())
    }

    #[tokio::test]
    async fn test_static_fetch_client() {
        let handler = |request: HttpRequestStream| {
            async move {
                let response = if let Some(true) = request
                    .headers
                    .get(CONVEX_CLIENT_HEADER)
                    .map(|v| v.eq(&*CONVEX_CLIENT_HEADER_VALUE))
                {
                    HttpResponse::new(
                        StatusCode::OK,
                        HeaderMap::new(),
                        Some("success".to_string().into_bytes()),
                        None,
                    )
                } else {
                    HttpResponse::new(
                        StatusCode::FORBIDDEN,
                        HeaderMap::new(),
                        Some("failed".to_string().into_bytes()),
                        None,
                    )
                };
                Ok(HttpResponseStream::from(response))
            }
            .boxed()
        };

        let url: url::Url = "https://google.ca".parse().unwrap();
        let mut fetch_client = StaticFetchClient::new();
        fetch_client.register_http_route(url.clone(), reqwest::Method::GET, handler);

        // Don't include Convex header
        let response = fetch_client
            .fetch(
                HttpRequest {
                    headers: HeaderMap::new(),
                    url: url.clone(),
                    method: http::Method::GET,
                    body: None,
                }
                .into(),
            )
            .await;
        let response = response.and_then(categorize_http_response_stream);
        assert!(response.is_err());
        assert!(response.err().unwrap().is_forbidden());

        // Include Convex header
        let response = fetch_client
            .fetch(
                HttpRequest {
                    headers: HeaderMap::from_iter([(
                        CONVEX_CLIENT_HEADER,
                        CONVEX_CLIENT_HEADER_VALUE.clone(),
                    )]),
                    url: url.clone(),
                    method: http::Method::GET,
                    body: None,
                }
                .into(),
            )
            .await
            .unwrap();

        let response = response.into_http_response().await.unwrap();
        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(
            String::from_utf8(response.body.unwrap()).unwrap(),
            "success"
        );
    }
}
