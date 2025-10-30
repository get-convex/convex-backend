use std::str::FromStr;

use common::{
    http::{
        HttpRequestStream,
        HttpResponse,
        HttpResponseStream,
    },
    sync::spsc,
};
use futures::{
    stream::BoxStream,
    FutureExt,
    StreamExt,
};
use headers::{
    HeaderMap,
    HeaderName,
};
use http::{
    HeaderValue,
    Method,
    StatusCode,
};
use serde::{
    Deserialize,
    Serialize,
};
use udf::HttpActionRequestHead;
use url::Url;

use crate::{
    ops::OpProvider,
    request_scope::StreamListener,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequestV8 {
    pub header_pairs: Vec<(String, String)>,
    pub url: String,
    pub method: String,
    pub stream_id: Option<uuid::Uuid>,
    pub signal: uuid::Uuid,
}

impl HttpRequestV8 {
    pub fn into_stream<'b, P: OpProvider<'b>>(
        self,
        provider: &mut P,
    ) -> anyhow::Result<HttpRequestStream> {
        let mut header_map = HeaderMap::new();
        for (name, value) in &self.header_pairs {
            header_map.append(HeaderName::from_str(name)?, byte_string_to_header(value)?);
        }
        let (body_sender, body_receiver) = spsc::unbounded_channel();
        match self.stream_id {
            Some(stream_id) => {
                provider.new_stream_listener(stream_id, StreamListener::RustStream(body_sender))?;
            },
            None => drop(body_sender),
        };
        let signal = {
            let (signal_sender, signal_receiver) = spsc::unbounded_channel();
            provider.new_stream_listener(self.signal, StreamListener::RustStream(signal_sender))?;
            let signal_stream = signal_receiver.into_stream();
            Box::pin(signal_stream.into_future().map(|_| ()))
        };

        Ok(HttpRequestStream {
            body: Box::pin(body_receiver.into_stream()),
            headers: header_map,
            url: Url::parse(&self.url)?,
            method: Method::from_str(&self.method)?,
            signal,
        })
    }

    pub fn from_request(
        request: HttpActionRequestHead,
        stream_id: Option<uuid::Uuid>,
        signal: uuid::Uuid,
    ) -> anyhow::Result<Self> {
        let mut header_pairs: Vec<(String, String)> = vec![];

        // Iterate over `&HeaderMap` instead of `HeaderMap` because the latter gives
        // None as the HeaderName for headers with multiple values
        // (https://docs.rs/http/latest/http/header/struct.HeaderMap.html#method.into_iter)
        for (name, value) in &request.headers {
            let value_str = header_to_byte_string(value);
            let header_name_str = name.as_str();
            header_pairs.push((header_name_str.to_string(), value_str));
        }

        Ok(Self {
            header_pairs,
            url: request.url.to_string(),
            method: request.method.to_string(),
            stream_id,
            signal,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponseV8 {
    stream_id: Option<uuid::Uuid>,
    status: u16,
    status_text: Option<String>,
    header_pairs: Vec<(String, String)>,
    url: Option<String>,
}

impl HttpResponseV8 {
    pub fn into_response(self) -> anyhow::Result<(HttpResponse, Option<uuid::Uuid>)> {
        let status_code = StatusCode::try_from(self.status)?;

        let mut header_map = HeaderMap::new();
        for (name, value) in &self.header_pairs {
            header_map.append(
                HeaderName::from_str(name.as_str())?,
                byte_string_to_header(value)?,
            );
        }

        Ok((
            HttpResponse {
                status: status_code,
                body: None,
                headers: header_map,
                url: self.url.map(|u| Url::parse(u.as_str())).transpose()?,
            },
            self.stream_id,
        ))
    }

    pub fn from_response_stream(
        mut response: HttpResponseStream,
        stream_id: uuid::Uuid,
    ) -> anyhow::Result<(
        Option<BoxStream<'static, anyhow::Result<bytes::Bytes>>>,
        HttpResponseV8,
    )> {
        let body = response.body.take();
        let mut header_pairs: Vec<(String, String)> = vec![];
        // Iterate over `&HeaderMap` instead of `HeaderMap` because the latter gives
        // None as the HeaderName for headers with multiple values
        // (https://docs.rs/http/latest/http/header/struct.HeaderMap.html#method.into_iter)
        for (name, value) in &response.headers {
            let value_str = header_to_byte_string(value);
            let header_name_str = name.as_str();
            header_pairs.push((header_name_str.to_string(), value_str));
        }
        // reqwest does not expose status text sent in HTTP response, so we derive it
        // from status code.
        let status_text = response
            .status
            .canonical_reason()
            .map(|reason| reason.to_string());
        Ok((
            body,
            HttpResponseV8 {
                stream_id: Some(stream_id),
                status: response.status.as_u16(),
                status_text,
                header_pairs,
                url: response.url.map(|u| u.to_string()),
            },
        ))
    }
}

// WebIDL ByteStrings use "isomorphic encoding" to convert to/from JS strings,
// i.e. latin-1
fn header_to_byte_string(header: &HeaderValue) -> String {
    header.as_bytes().iter().map(|&b| char::from(b)).collect()
}

fn byte_string_to_header(header: &str) -> anyhow::Result<HeaderValue> {
    // TODO: turn these into TypeErrors
    let bytes = header
        .chars()
        .map(|c| u8::try_from(c).map_err(|_| anyhow::anyhow!("invalid char for header: `{c}`")))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(HeaderValue::from_bytes(&bytes)?)
}
