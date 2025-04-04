#![feature(try_blocks)]
#![feature(impl_trait_in_fn_trait_return)]

use std::sync::LazyLock;

use bytes::BufMut;
use futures::Future;
use http_body_util::BodyExt;
use http_cache::XCACHE;
use http_cache_reqwest::{
    Cache,
    CacheMode,
    HttpCache,
    MokaManager,
};
use metrics::log_http_response;
use openidconnect::{
    http,
    HttpRequest,
    HttpResponse,
};
use reqwest::Client;
use reqwest_middleware::ClientBuilder;
use thiserror::Error;

mod metrics;

#[derive(Error, Debug)]
#[error(transparent)]
pub struct AsStdError(#[from] anyhow::Error);

static CACHE: LazyLock<MokaManager> = LazyLock::new(MokaManager::default);
static HTTP_CLIENT: LazyLock<reqwest_middleware::ClientWithMiddleware> = LazyLock::new(|| {
    ClientBuilder::new(Client::new())
        .with(Cache(HttpCache {
            mode: CacheMode::Default,
            manager: CACHE.clone(),
            options: Default::default(),
        }))
        .build()
});

/// Just for metrics labeling
#[derive(Copy, Clone, Eq, PartialEq, Debug, strum::IntoStaticStr)]
pub enum ClientPurpose {
    ProviderMetadata,
    Jwks,
    UserInfo,
}

pub fn cached_http_client_for(
    purpose: ClientPurpose,
) -> impl Fn(HttpRequest) -> (impl Future<Output = Result<HttpResponse, AsStdError>> + 'static) {
    move |request: HttpRequest| cached_http_client_inner(request, purpose)
}

/// HTTP fetch function that caches responses in memory based on the
/// `Cache-Control` headers in the response.
/// Uses a static `reqwest` client so connections can be reused.
async fn cached_http_client_inner(
    request: HttpRequest,
    purpose: ClientPurpose,
) -> Result<HttpResponse, AsStdError> {
    // Error handling shenanigans because `anyhow::Error` doesn't implement
    // `std::error::Error` (required by openidconnect), but the function body
    // returns multiple error types that are easiest to unify under
    // `anyhow::Error`. We can collect the result as an `anyhow::Error`, then
    // convert it to a `AsStdError` which does implement `std::error::Error
    let res: Result<HttpResponse, anyhow::Error> = try {
        let (parts, body) = request.into_parts();
        let mut request_builder = HTTP_CLIENT
            .request(parts.method.as_str().parse()?, parts.uri.to_string())
            .body(body);
        for (name, value) in &parts.headers {
            request_builder = request_builder.header(name.as_str(), value.as_bytes());
        }
        let request = request_builder.build()?;

        let response = HTTP_CLIENT.execute(request).await?;

        let cache_hit = response
            .headers()
            .get(XCACHE)
            .map(|header| header.as_bytes() == "HIT".as_bytes())
            .unwrap_or_default();

        log_http_response(purpose, cache_hit);

        // `openidconnect` requires that the response be a single `Vec<u8>`
        // chunk, so read the entire response body here
        let (parts, body) = http::Response::from(response).into_parts();
        let body = body.collect().await?;
        let mut vec_body = vec![];
        vec_body.put(body.aggregate());
        HttpResponse::from_parts(parts, vec_body)
    };
    res.map_err(AsStdError)
}

#[cfg(test)]
mod tests {
    use http_cache::XCACHE;
    use openidconnect::http::{
        self,
        header::HeaderValue,
    };
    use reqwest::Url;

    use crate::{
        cached_http_client_inner,
        ClientPurpose,
    };

    #[tokio::test]
    async fn test_cached_client() -> anyhow::Result<()> {
        // Use Google's OpenID configuration, which should never disappear
        let url =
            Url::parse("https://accounts.google.com/.well-known/openid-configuration").unwrap();
        let request = http::Request::get(url.to_string())
            .header(
                http::header::ACCEPT,
                HeaderValue::from_static("application/json"),
            )
            .body(vec![])?;
        let response = cached_http_client_inner(request.clone(), ClientPurpose::ProviderMetadata)
            .await
            .unwrap();
        assert_eq!(
            response.headers().get(XCACHE).unwrap().as_bytes(),
            "MISS".as_bytes()
        );
        // Send the request again
        let response = cached_http_client_inner(request, ClientPurpose::ProviderMetadata)
            .await
            .unwrap();
        assert_eq!(
            response.headers().get(XCACHE).unwrap().as_bytes(),
            "HIT".as_bytes()
        );
        Ok(())
    }
}
