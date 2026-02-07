use std::str::FromStr;

use common::sync::spsc;
use deno_core::{
    serde_v8,
    v8::{
        self,
    },
};
use headers::HeaderName;
use serde::{
    Deserialize,
    Serialize,
};
use url::{
    form_urlencoded,
    Position,
    Url,
};

use super::OpProvider;
use crate::{
    convert_v8::TypeError,
    environment::{
        helpers::with_argument_error,
        AsyncOpRequest,
    },
    http::HttpRequestV8,
    request_scope::StreamListener,
};

pub fn async_op_fetch<'b, P: OpProvider<'b>>(
    provider: &mut P,
    args: v8::FunctionCallbackArguments,
    resolver: v8::Global<v8::PromiseResolver>,
) -> anyhow::Result<()> {
    let arg: HttpRequestV8 = serde_v8::from_v8(&mut provider.scope(), args.get(1))?;

    let request = with_argument_error("fetch", || HttpRequestV8::into_stream(arg, provider))?;
    let response_body_stream_id = provider.create_stream()?;
    provider.start_async_op(
        AsyncOpRequest::Fetch {
            request,
            response_body_stream_id,
        },
        resolver,
    )
}

pub fn async_op_parse_multi_part<'b, P: OpProvider<'b>>(
    provider: &mut P,
    args: v8::FunctionCallbackArguments,
    resolver: v8::Global<v8::PromiseResolver>,
) -> anyhow::Result<()> {
    let content_type: String = serde_v8::from_v8(&mut provider.scope(), args.get(1))?;
    let request_stream_id: uuid::Uuid = serde_v8::from_v8(&mut provider.scope(), args.get(2))?;
    let (request_sender, request_receiver) = spsc::unbounded_channel();
    provider.new_stream_listener(
        request_stream_id,
        StreamListener::RustStream(request_sender),
    )?;

    provider.start_async_op(
        AsyncOpRequest::ParseMultiPart {
            content_type,
            request_stream: Box::pin(request_receiver.into_stream()),
        },
        resolver,
    )
}

#[convex_macro::v8_op]
pub fn op_url_get_url_info<'b, P: OpProvider<'b>>(
    provider: &mut P,
    url: String,
    base: Option<String>,
) -> anyhow::Result<UrlInfo> {
    let base_url = match base {
        Some(b) => match Url::parse(&b) {
            Ok(url) => Some(url),
            Err(_) => anyhow::bail!(TypeError::new(format!("Invalid URL: '{b}'"))),
        },
        None => None,
    };

    let parsed_url = match Url::options().base_url(base_url.as_ref()).parse(&url) {
        Ok(u) => u,
        // The URL spec (https://url.spec.whatwg.org/) dictates that JS
        // throw a TypeError when the URL is invalid.
        Err(_) => anyhow::bail!(TypeError::new(format!("Invalid URL: '{url}'"))),
    };
    Ok(UrlInfo::from(parsed_url))
}

#[convex_macro::v8_op]
pub fn op_url_get_url_search_param_pairs<'b, P: OpProvider<'b>>(
    provider: &mut P,
    query_string: String,
) -> anyhow::Result<Vec<(String, String)>> {
    let parsed_url = form_urlencoded::parse(query_string.as_bytes());

    let query_pairs: Vec<(String, String)> = parsed_url
        .into_iter()
        .map(|(key, value)| (key.into(), value.into()))
        .collect();

    Ok(query_pairs)
}

#[convex_macro::v8_op]
pub fn op_url_stringify_url_search_params<'b, P: OpProvider<'b>>(
    provider: &mut P,
    query_pairs: Vec<(String, String)>,
) -> anyhow::Result<String> {
    let search = form_urlencoded::Serializer::new(String::new())
        .extend_pairs(query_pairs)
        .finish();
    Ok(search)
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
enum UrlInfoUpdate {
    Hash(Option<String>),
    Hostname(Option<String>),
    Href(String),
    Protocol(String),
    Port(String),
    Pathname(String),
    Search(Option<String>),
    SearchParams(Vec<(String, String)>),
}

#[convex_macro::v8_op]
pub fn op_url_update_url_info<'b, P: OpProvider<'b>>(
    provider: &mut P,
    original_url: String,
    update: UrlInfoUpdate,
) -> anyhow::Result<UrlInfo> {
    let mut parsed_url = Url::parse(&original_url)?;

    match update {
        UrlInfoUpdate::Hash(value) => parsed_url.set_fragment(value.as_deref()),
        UrlInfoUpdate::SearchParams(value) => {
            if value.is_empty() {
                parsed_url.set_query(None)
            } else {
                parsed_url
                    .query_pairs_mut()
                    .clear()
                    .extend_pairs(value)
                    .finish();
            }
        },
        UrlInfoUpdate::Hostname(value) => parsed_url.set_host(value.as_deref())?,
        UrlInfoUpdate::Href(value) => {
            parsed_url = Url::parse(&value)
                .map_err(|_| TypeError::new(format!("Could not parse URL: {original_url}")))?;
        },
        UrlInfoUpdate::Protocol(value) => {
            // ignore errors
            _ = parsed_url.set_scheme(&value);
        },
        UrlInfoUpdate::Port(port_str) => {
            // ignore errors
            if port_str.is_empty() {
                _ = parsed_url.set_port(None);
            } else if let Ok(port) = port_str.parse::<u16>() {
                _ = parsed_url.set_port(Some(port));
            }
        },
        UrlInfoUpdate::Pathname(value) => parsed_url.set_path(&value),
        UrlInfoUpdate::Search(value) => parsed_url.set_query(value.as_deref()),
    }

    Ok(UrlInfo::from(parsed_url))
}

#[convex_macro::v8_op]
pub fn op_headers_get_mime_type<'b, P: OpProvider<'b>>(
    provider: &mut P,
    content_type: String,
) -> anyhow::Result<Option<MimeType>> {
    let mime_type = match mime::Mime::from_str(&content_type) {
        Ok(mime_type) => mime_type,
        Err(_) => {
            // Invalid mime type, so turn it into null on the JS side.
            return Ok(None);
        },
    };
    Ok(Some(MimeType {
        essence: mime_type.essence_str().to_string(),
        boundary: mime_type.get_param(mime::BOUNDARY).map(|b| b.to_string()),
    }))
}

#[convex_macro::v8_op]
pub fn op_headers_normalize_name<'b, P: OpProvider<'b>>(
    provider: &mut P,
    name: String,
) -> anyhow::Result<Option<String>> {
    let result = match HeaderName::from_bytes(name.as_bytes()) {
        Ok(normalized_name) => Some(normalized_name.as_str().into()),
        // This is an invalid header name, so turn it into a TypeError on the
        // JS side
        Err(_) => None,
    };
    Ok(result)
}

#[derive(Deserialize, Serialize)]
struct UrlInfo {
    scheme: String,
    hash: String,
    host: String,
    hostname: String,
    href: String,
    pathname: String,
    port: String,
    search: String,
    username: String,
    password: String,
}

impl From<Url> for UrlInfo {
    fn from(value: Url) -> Self {
        UrlInfo {
            scheme: value.scheme().to_string(),
            hash: value[Position::BeforeFragment..Position::AfterFragment].to_string(),
            host: value[Position::BeforeHost..Position::BeforePath].to_string(),
            hostname: value[Position::BeforeHost..Position::AfterHost].to_string(),
            href: value.to_string(),
            pathname: value[Position::BeforePath..Position::AfterPath].to_string(),
            port: value[Position::BeforePort..Position::AfterPort].to_string(),
            search: value[Position::BeforeQuery..Position::AfterQuery].to_string(),
            username: value.username().to_owned(),
            password: value.password().unwrap_or_default().to_owned(),
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MimeType {
    essence: String,
    boundary: Option<String>,
}
