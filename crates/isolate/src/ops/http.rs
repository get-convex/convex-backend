use std::str::FromStr;

use anyhow::Context;
use common::runtime::Runtime;
use deno_core::{
    serde_v8,
    v8::{
        self,
    },
};
use errors::ErrorMetadata;
use futures::channel::mpsc;
use headers::HeaderName;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::{
    json,
    Value as JsonValue,
};
use url::{
    form_urlencoded,
    Position,
    Url,
};

use crate::{
    environment::{
        helpers::with_argument_error,
        AsyncOpRequest,
        IsolateEnvironment,
    },
    execution_scope::ExecutionScope,
    http::HttpRequestV8,
    request_scope::StreamListener,
};

impl<'a, 'b: 'a, RT: Runtime, E: IsolateEnvironment<RT>> ExecutionScope<'a, 'b, RT, E> {
    pub fn async_op_fetch(
        &mut self,
        args: v8::FunctionCallbackArguments,
        resolver: v8::Global<v8::PromiseResolver>,
    ) -> anyhow::Result<()> {
        let arg: HttpRequestV8 = serde_v8::from_v8(self, args.get(1))?;

        let request = with_argument_error("fetch", || HttpRequestV8::into_stream(arg, self))?;
        let state = self.state_mut()?;
        let response_body_stream_id = state.create_stream()?;
        state.environment.start_async_op(
            AsyncOpRequest::Fetch {
                request,
                response_body_stream_id,
            },
            resolver,
        )
    }

    pub fn async_op_parseMultiPart(
        &mut self,
        args: v8::FunctionCallbackArguments,
        resolver: v8::Global<v8::PromiseResolver>,
    ) -> anyhow::Result<()> {
        let content_type: String = serde_v8::from_v8(self, args.get(1))?;
        let request_stream_id: uuid::Uuid = serde_v8::from_v8(self, args.get(2))?;
        let (request_sender, request_receiver) = mpsc::unbounded();
        self.new_stream_listener(
            request_stream_id,
            StreamListener::RustStream(request_sender),
        )?;

        let state = self.state_mut()?;
        state.environment.start_async_op(
            AsyncOpRequest::ParseMultiPart {
                content_type,
                request_stream: Box::pin(request_receiver),
            },
            resolver,
        )
    }

    #[convex_macro::v8_op]
    pub fn op_url_getUrlInfo(
        &mut self,
        url: String,
        base: Option<String>,
    ) -> anyhow::Result<JsonValue> {
        let base_url = match base {
            Some(b) => match Url::parse(&b) {
                Ok(url) => Some(url),
                Err(_) => return Ok(json!({"kind": "error", "errorType": "InvalidURL"})),
            },
            None => None,
        };

        let parsed_url = match Url::options().base_url(base_url.as_ref()).parse(&url) {
            Ok(u) => u,
            // The URL spec (https://url.spec.whatwg.org/) dictates that JS
            // throw a TypeError when the URL is invalid, so we return `null`
            // and throw the error on the JS side instead of having this error
            // turn into a JsError
            Err(_) => return Ok(json!({"kind": "error", "errorType": "InvalidURL"})),
        };
        match UrlInfo::try_from(parsed_url) {
            Ok(url_info) => {
                Ok(json!({"kind": "success", "urlInfo": serde_json::to_value(url_info)?}))
            },
            // This is a valid URL, but not one we support
            Err(e) => Ok(
                json!({"kind": "error", "errorType": "UnsupportedURL", "message": e.to_string()}),
            ),
        }
    }

    #[convex_macro::v8_op]
    pub fn op_url_getUrlSearchParamPairs(
        &mut self,
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
    pub fn op_url_stringifyUrlSearchParams(
        &mut self,
        query_pairs: Vec<(String, String)>,
    ) -> anyhow::Result<String> {
        let search = form_urlencoded::Serializer::new(String::new())
            .extend_pairs(query_pairs)
            .finish();
        Ok(search)
    }

    #[convex_macro::v8_op]
    pub fn op_url_updateUrlInfo(
        &mut self,
        original_url: String,
        update: JsonValue,
    ) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize, Debug, Clone)]
        #[serde(rename_all = "camelCase")]
        #[serde(tag = "type")]
        enum Update {
            Hash { value: Option<String> },
            Hostname { value: Option<String> },
            Href { value: String },
            Protocol { value: String },
            Port { value: Option<String> },
            Pathname { value: String },
            Search { value: Option<String> },
            SearchParams { value: Vec<(String, String)> },
        }

        let update: Update = serde_json::from_value(update)?;
        let mut parsed_url = Url::parse(&original_url)?;

        match update {
            Update::Hash { value } => parsed_url.set_fragment(value.as_deref()),
            Update::SearchParams { value } => {
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
            Update::Hostname { value } => parsed_url.set_host(value.as_deref())?,
            Update::Href { value } => {
                parsed_url = Url::parse(&value).context(ErrorMetadata::bad_request(
                    "BadUrl",
                    format!("Could not parse URL: {original_url}"),
                ))?;
            },
            Update::Protocol { value } => {
                if value != "http" && value != "https" {
                    parsed_url
                        .set_scheme(&value)
                        .map_err(|_e| anyhow::anyhow!("Failed to set scheme"))?
                }
            },
            Update::Port { value } => match value {
                Some(port_str) => match port_str.parse::<u16>() {
                    Ok(port_number) => parsed_url
                        .set_port(Some(port_number))
                        .map_err(|_e| anyhow::anyhow!("Failed to set port"))?,
                    Err(_) => (),
                },
                None => parsed_url
                    .set_port(None)
                    .map_err(|_e| anyhow::anyhow!("Failed to set port"))?,
            },
            Update::Pathname { value } => parsed_url.set_path(&value),
            Update::Search { value } => parsed_url.set_query(value.as_deref()),
        }

        let url_info: UrlInfo = parsed_url.try_into()?;
        Ok(serde_json::to_value(url_info)?)
    }

    #[convex_macro::v8_op]
    pub fn op_headers_getMimeType(
        &mut self,
        content_type: String,
    ) -> anyhow::Result<Option<MimeType>> {
        let Ok(mime_type) = mime::Mime::from_str(&content_type) else {
            // Invalid mime type, so turn it into null on the JS side.
            return Ok(None);
        };
        Ok(Some(MimeType {
            essence: mime_type.essence_str().to_string(),
            boundary: mime_type.get_param(mime::BOUNDARY).map(|b| b.to_string()),
        }))
    }

    #[convex_macro::v8_op]
    pub fn op_headers_normalizeName(&mut self, name: String) -> anyhow::Result<Option<String>> {
        let result = match HeaderName::from_bytes(name.as_bytes()) {
            Ok(normalized_name) => Some(normalized_name.as_str().into()),
            // This is an invalid header name, so turn it into a TypeError on the
            // JS side
            Err(_) => None,
        };
        Ok(result)
    }
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
    protocol: String,
    search: String,
}

impl TryFrom<Url> for UrlInfo {
    type Error = anyhow::Error;

    fn try_from(value: Url) -> Result<Self, Self::Error> {
        if value.username() != "" || value.password().is_some() {
            anyhow::bail!("Unsupported URL with username and password")
        }

        if value.scheme() != "http" && value.scheme() != "https" {
            anyhow::bail!(
                "Unsupported URL scheme -- http and https are supported (scheme was {})",
                value.scheme()
            )
        }

        let url_info = UrlInfo {
            scheme: value[Position::BeforeScheme..Position::AfterScheme].to_string(),
            hash: value[Position::BeforeFragment..Position::AfterFragment].to_string(),
            host: value[Position::BeforeHost..Position::BeforePath].to_string(),
            hostname: value[Position::BeforeHost..Position::AfterHost].to_string(),
            href: value.to_string(),
            pathname: value[Position::BeforePath..Position::AfterPath].to_string(),
            port: value[Position::BeforePort..Position::AfterPort].to_string(),
            protocol: value[..Position::AfterScheme].to_string(),
            search: value[Position::BeforeQuery..Position::AfterQuery].to_string(),
        };
        Ok(url_info)
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MimeType {
    essence: String,
    boundary: Option<String>,
}
