#![feature(try_blocks)]
#![feature(impl_trait_in_fn_trait_return)]

use std::sync::LazyLock;

use futures::Future;
use http_cache::XCACHE;
use http_cache_reqwest::{
    Cache,
    CacheMode,
    HttpCache,
    MokaManager,
};
use metrics::log_http_response;
use openidconnect::{
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
        let mut request_builder = HTTP_CLIENT
            .request(request.method.as_str().parse()?, request.url.as_str())
            .body(request.body);
        for (name, value) in &request.headers {
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

        let status_code = response.status();
        let headers = response.headers().to_owned();
        let chunks = response.bytes().await?;
        HttpResponse {
            status_code: status_code.as_str().parse()?,
            headers: headers
                .iter()
                .map(|(name, value)| {
                    Ok((
                        openidconnect::http::HeaderName::from_bytes(name.as_ref())?,
                        openidconnect::http::HeaderValue::from_bytes(value.as_bytes())?,
                    ))
                })
                .collect::<anyhow::Result<_>>()?,
            body: chunks.to_vec(),
        }
    };
    res.map_err(AsStdError)
}

#[cfg(test)]
mod tests {
    use http_cache::XCACHE;
    use openidconnect::{
        http::{
            header::HeaderValue,
            Method,
        },
        HttpRequest,
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
        let request = HttpRequest {
            url: url.clone(),
            method: Method::GET,
            headers: vec![(
                openidconnect::http::header::ACCEPT,
                HeaderValue::from_static("application/json"),
            )]
            .into_iter()
            .collect(),
            body: vec![],
        };
        let response = cached_http_client_inner(request.clone(), ClientPurpose::ProviderMetadata)
            .await
            .unwrap();
        assert_eq!(
            response.headers.get(XCACHE).unwrap().as_bytes(),
            "MISS".as_bytes()
        );
        // Send the request again
        let response = cached_http_client_inner(request, ClientPurpose::ProviderMetadata)
            .await
            .unwrap();
        assert_eq!(
            response.headers.get(XCACHE).unwrap().as_bytes(),
            "HIT".as_bytes()
        );
        Ok(())
    }
}
