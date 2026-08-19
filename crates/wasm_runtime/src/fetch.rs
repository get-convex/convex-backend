use std::sync::OnceLock;

use crossbeam_channel::Sender;
use reqwest::{
    redirect,
    Proxy,
};
use serde::{
    Deserialize,
    Serialize,
};
use tokio::runtime::{
    Builder,
    Runtime,
};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchRequest {
    pub url: String,
    pub method: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResponse {
    pub status: u16,
    pub ok: bool,
    pub url: Option<String>,
    pub headers: Vec<(String, String)>,
    pub body_text: Option<String>,
}

#[derive(Debug)]
pub struct FetchCompletion {
    pub op_id: i32,
    pub result_json: Result<String, String>,
}

static FETCH_RUNTIME: OnceLock<Runtime> = OnceLock::new();
static FETCH_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn fetch_runtime() -> &'static Runtime {
    FETCH_RUNTIME.get_or_init(|| {
        let worker_threads = std::thread::available_parallelism()
            .map(|value| value.get().max(2))
            .unwrap_or(2);
        Builder::new_multi_thread()
            .worker_threads(worker_threads)
            .enable_all()
            .build()
            .expect("shared fetch runtime should initialize")
    })
}

fn build_client(proxy_url: Option<Url>) -> reqwest::Client {
    let mut builder = reqwest::Client::builder().redirect(redirect::Policy::none());
    if let Some(proxy_url) = proxy_url {
        let proxy = Proxy::all(proxy_url).expect("proxy url should build");
        builder = builder.proxy(proxy);
    }
    builder = builder.user_agent("Convex/1.0");
    builder.build().expect("failed to build reqwest client")
}

fn fetch_client() -> &'static reqwest::Client {
    FETCH_CLIENT.get_or_init(|| build_client(None))
}

pub fn spawn_fetch(op_id: i32, request: FetchRequest, completion_tx: Sender<FetchCompletion>) {
    fetch_runtime().spawn(async move {
        let result_json = run_fetch(request).await.and_then(|response| {
            serde_json::to_string(&response).map_err(|error| error.to_string())
        });

        let _ = completion_tx.send(FetchCompletion { op_id, result_json });
    });
}

pub fn parse_fetch_request(request_json: &str) -> Result<FetchRequest, String> {
    serde_json::from_str(request_json).map_err(|error| format!("invalid fetch request: {error}"))
}

async fn run_fetch(request: FetchRequest) -> Result<FetchResponse, String> {
    let method = reqwest::Method::from_bytes(request.method.as_bytes())
        .map_err(|error| format!("invalid HTTP method: {error}"))?;

    let mut request_builder = fetch_client().request(method, &request.url);
    if let Some(body) = request.body {
        request_builder = request_builder.body(body);
    }

    for (name, value) in request.headers {
        request_builder = request_builder.header(name, value);
    }

    let raw_request = request_builder
        .build()
        .map_err(|error| format!("failed to build fetch request: {error}"))?;
    let raw_response = fetch_client()
        .execute(raw_request)
        .await
        .map_err(|error| format!("fetch failed: {error}"))?;

    if raw_response.status() == reqwest::StatusCode::PROXY_AUTHENTICATION_REQUIRED {
        return Err(format!("Request to {} forbidden", request.url));
    }

    let status = raw_response.status();
    let headers = raw_response
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_owned(),
                value.to_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    let url = Some(raw_response.url().to_string());
    let body_text = Some(
        raw_response
            .text()
            .await
            .map_err(|error| format!("failed to read fetch response body: {error}"))?,
    );

    Ok(FetchResponse {
        status: status.as_u16(),
        ok: status.is_success(),
        url,
        headers,
        body_text,
    })
}
