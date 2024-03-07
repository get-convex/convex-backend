#![feature(try_blocks)]
#![feature(lazy_cell)]

use std::sync::LazyLock;

use http_cache_reqwest::{
    Cache,
    CacheMode,
    HttpCache,
    MokaManager,
};
use openidconnect::{
    HttpRequest,
    HttpResponse,
};
use reqwest::Client;
use reqwest_middleware::ClientBuilder;
use thiserror::Error;

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

/// HTTP fetch function that caches responses in memory based on the
/// `Cache-Control` headers in the response.
/// Uses a static `reqwest` client so connections can be reused.
pub async fn cached_http_client(request: HttpRequest) -> Result<HttpResponse, AsStdError> {
    // Error handling shenanigans because `anyhow::Error` doesn't implement
    // `std::error::Error` (required by openidconnect), but the function body
    // returns multiple error types that are easiest to unify under
    // `anyhow::Error`. We can collect the result as an `anyhow::Error`, then
    // convert it to a `AsStdError` which does implement `std::error::Error
    let res: Result<HttpResponse, anyhow::Error> = try {
        let mut request_builder = HTTP_CLIENT
            .request(request.method, request.url.as_str())
            .body(request.body);
        for (name, value) in &request.headers {
            request_builder = request_builder.header(name.as_str(), value.as_bytes());
        }
        let request = request_builder.build()?;

        let response = HTTP_CLIENT.execute(request).await?;

        let status_code = response.status();
        let headers = response.headers().to_owned();
        let chunks = response.bytes().await?;
        HttpResponse {
            status_code,
            headers,
            body: chunks.to_vec(),
        }
    };
    res.map_err(AsStdError)
}
