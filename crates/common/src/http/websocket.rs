//! Forked from axum's WebSocket extractor to add permessage-deflate support.
//! https://github.com/tokio-rs/axum/blob/main/axum/src/extract/ws.rs
//!
//! MIT Licensed https://github.com/tokio-rs/axum?tab=readme-ov-file#license
//!
//! Handle WebSocket connections.
//!
//! Convex fork deltas from upstream `axum::extract::ws` are commented below
//! with `// Convex fork`:
//! - Rejection type is `HttpResponseError` + `ErrorMetadata` instead of Axum's
//!   `WebSocketUpgradeRejection` enum/macros.
//! - Adds inbound `permessage-deflate` support by parsing
//!   `Sec-WebSocket-Extensions`, enabling
//!   `WebSocketConfig.extensions.permessage_deflate`, and echoing
//!   `Sec-WebSocket-Extensions: permessage-deflate` in the 101 response.
//! - Uses the workspace base64 API (`base64 = 0.13`) in `sign()`.

use std::{
    borrow::Cow,
    collections::BTreeSet,
    future::Future,
    pin::Pin,
    task::{
        ready,
        Context,
        Poll,
    },
};

use anyhow::anyhow;
use axum::{
    body::Body,
    extract::FromRequestParts,
    response::Response,
    Error,
};
use bytes::Bytes;
use errors::ErrorMetadata;
use futures::{
    stream::FusedStream,
    Sink,
    Stream,
};
use futures_util::{
    sink::SinkExt,
    stream::StreamExt,
};
use http::{
    header::{
        self,
        HeaderMap,
        HeaderName,
        HeaderValue,
    },
    request::Parts,
    Method,
    StatusCode,
    Version,
};
use hyper_util::rt::TokioIo;
use sha1::{
    Digest,
    Sha1,
};
use tokio_tungstenite::{
    tungstenite::{
        self as ts,
        extensions::compression::deflate::DeflateConfig,
        protocol::{
            self,
            WebSocketConfig,
        },
    },
    WebSocketStream,
};

use crate::http::HttpResponseError;

#[must_use]
pub struct WebSocketUpgrade<F = DefaultOnFailedUpgrade> {
    config: WebSocketConfig,
    protocol: Option<HeaderValue>,
    sec_websocket_key: Option<HeaderValue>,
    on_upgrade: hyper::upgrade::OnUpgrade,
    on_failed_upgrade: F,
    sec_websocket_protocol: BTreeSet<HeaderValue>,
    // Convex fork: remembers whether the client offered permessage-deflate.
    client_offers_deflate: bool,
}

impl<F> std::fmt::Debug for WebSocketUpgrade<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebSocketUpgrade")
            .field("config", &self.config)
            .field("protocol", &self.protocol)
            .field("sec_websocket_key", &self.sec_websocket_key)
            .field("sec_websocket_protocol", &self.sec_websocket_protocol)
            .field("client_offers_deflate", &self.client_offers_deflate)
            .finish_non_exhaustive()
    }
}

impl<F> WebSocketUpgrade<F> {
    pub fn read_buffer_size(mut self, size: usize) -> Self {
        self.config.read_buffer_size = size;
        self
    }

    pub fn write_buffer_size(mut self, size: usize) -> Self {
        self.config.write_buffer_size = size;
        self
    }

    pub fn max_write_buffer_size(mut self, max: usize) -> Self {
        self.config.max_write_buffer_size = max;
        self
    }

    pub fn max_message_size(mut self, max: usize) -> Self {
        self.config.max_message_size = Some(max);
        self
    }

    pub fn max_frame_size(mut self, max: usize) -> Self {
        self.config.max_frame_size = Some(max);
        self
    }

    pub fn accept_unmasked_frames(mut self, accept: bool) -> Self {
        self.config.accept_unmasked_frames = accept;
        self
    }

    pub fn protocols<I>(mut self, protocols: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Cow<'static, str>>,
    {
        self.protocol = protocols
            .into_iter()
            .map(Into::into)
            .find(|proto| {
                let Ok(proto) = HeaderValue::from_str(proto) else {
                    return false;
                };
                self.sec_websocket_protocol.contains(&proto)
            })
            .map(|protocol| match protocol {
                Cow::Owned(s) => HeaderValue::from_str(&s).unwrap(),
                Cow::Borrowed(s) => HeaderValue::from_static(s),
            });

        self
    }

    pub fn requested_protocols(&self) -> impl Iterator<Item = &HeaderValue> {
        self.sec_websocket_protocol.iter()
    }

    pub fn set_selected_protocol(&mut self, protocol: HeaderValue) {
        self.protocol = Some(protocol);
    }

    pub fn selected_protocol(&self) -> Option<&HeaderValue> {
        self.protocol.as_ref()
    }

    pub fn on_failed_upgrade<C>(self, callback: C) -> WebSocketUpgrade<C>
    where
        C: OnFailedUpgrade,
    {
        WebSocketUpgrade {
            config: self.config,
            protocol: self.protocol,
            sec_websocket_key: self.sec_websocket_key,
            on_upgrade: self.on_upgrade,
            on_failed_upgrade: callback,
            sec_websocket_protocol: self.sec_websocket_protocol,
            client_offers_deflate: self.client_offers_deflate,
        }
    }

    #[must_use = "to set up the WebSocket connection, this response must be returned"]
    pub fn on_upgrade<C, Fut>(self, callback: C) -> Response
    where
        C: FnOnce(WebSocket) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
        F: OnFailedUpgrade,
    {
        let on_upgrade = self.on_upgrade;
        let mut config = self.config;
        let on_failed_upgrade = self.on_failed_upgrade;
        let protocol = self.protocol.clone();
        let client_offers_deflate = self.client_offers_deflate;

        tokio::spawn(async move {
            let upgraded = match on_upgrade.await {
                Ok(upgraded) => upgraded,
                Err(err) => {
                    on_failed_upgrade.call(Error::new(err));
                    return;
                },
            };
            let upgraded = TokioIo::new(upgraded);

            if client_offers_deflate {
                config.extensions.permessage_deflate = Some(DeflateConfig::default());
            }

            let socket =
                WebSocketStream::from_raw_socket(upgraded, protocol::Role::Server, Some(config))
                    .await;
            let socket = WebSocket {
                inner: socket,
                protocol,
            };
            callback(socket).await;
        });

        let mut response = if let Some(sec_websocket_key) = &self.sec_websocket_key {
            #[allow(clippy::declare_interior_mutable_const)]
            const UPGRADE: HeaderValue = HeaderValue::from_static("upgrade");
            #[allow(clippy::declare_interior_mutable_const)]
            const WEBSOCKET: HeaderValue = HeaderValue::from_static("websocket");

            Response::builder()
                .status(StatusCode::SWITCHING_PROTOCOLS)
                .header(header::CONNECTION, UPGRADE)
                .header(header::UPGRADE, WEBSOCKET)
                .header(
                    header::SEC_WEBSOCKET_ACCEPT,
                    sign(sec_websocket_key.as_bytes()),
                )
                .body(Body::empty())
                .unwrap()
        } else {
            Response::new(Body::empty())
        };

        if let Some(protocol) = self.protocol {
            response
                .headers_mut()
                .insert(header::SEC_WEBSOCKET_PROTOCOL, protocol);
        }
        // Convex fork: if the client offered permessage-deflate, advertise support.
        if self.client_offers_deflate {
            response.headers_mut().insert(
                HeaderName::from_static("sec-websocket-extensions"),
                HeaderValue::from_static("permessage-deflate"),
            );
        }

        response
    }
}

pub trait OnFailedUpgrade: Send + 'static {
    fn call(self, error: Error);
}

impl<F> OnFailedUpgrade for F
where
    F: FnOnce(Error) + Send + 'static,
{
    fn call(self, error: Error) {
        self(error)
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct DefaultOnFailedUpgrade;

impl OnFailedUpgrade for DefaultOnFailedUpgrade {
    #[inline]
    fn call(self, _error: Error) {}
}

impl<S> FromRequestParts<S> for WebSocketUpgrade<DefaultOnFailedUpgrade>
where
    S: Send + Sync,
{
    // Convex fork: map extractor failures into HttpResponseError.
    type Rejection = HttpResponseError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let sec_websocket_key = if parts.version <= Version::HTTP_11 {
            if parts.method != Method::GET {
                return Err(anyhow!(ErrorMetadata::bad_request(
                    "MethodNotGet",
                    "Request method must be GET",
                ))
                .into());
            }
            if !header_contains(&parts.headers, &header::CONNECTION, "upgrade") {
                return Err(anyhow!(ErrorMetadata::bad_request(
                    "InvalidConnectionHeader",
                    "Connection header did not include upgrade",
                ))
                .into());
            }
            if !header_eq(&parts.headers, &header::UPGRADE, "websocket") {
                return Err(anyhow!(ErrorMetadata::bad_request(
                    "InvalidUpgradeHeader",
                    "Upgrade header did not include websocket",
                ))
                .into());
            }
            Some(
                parts
                    .headers
                    .get(header::SEC_WEBSOCKET_KEY)
                    .ok_or_else(|| {
                        anyhow!(ErrorMetadata::bad_request(
                            "WebSocketKeyHeaderMissing",
                            "Sec-WebSocket-Key header missing",
                        ))
                    })?
                    .clone(),
            )
        } else {
            if parts.method != Method::CONNECT {
                return Err(anyhow!(ErrorMetadata::bad_request(
                    "MethodNotConnect",
                    "Request method must be CONNECT",
                ))
                .into());
            }
            None
        };

        if !header_eq(&parts.headers, &header::SEC_WEBSOCKET_VERSION, "13") {
            return Err(anyhow!(ErrorMetadata::bad_request(
                "InvalidWebSocketVersionHeader",
                "Sec-WebSocket-Version header did not include 13",
            ))
            .into());
        }

        let on_upgrade = parts
            .extensions
            .remove::<hyper::upgrade::OnUpgrade>()
            .ok_or_else(|| {
                anyhow!(ErrorMetadata::bad_request(
                    "ConnectionNotUpgradable",
                    "WebSocket request could not be upgraded",
                ))
            })?;

        let sec_websocket_protocol = parts
            .headers
            .get_all(header::SEC_WEBSOCKET_PROTOCOL)
            .iter()
            .flat_map(|val| val.as_bytes().split(|&b| b == b','))
            .map(|proto| {
                HeaderValue::from_bytes(trim_ascii(proto))
                    .expect("substring of HeaderValue is valid HeaderValue")
            })
            .collect();

        // Convex fork: parse extension offers to opt into permessage-deflate.
        //
        // NOTE: We intentionally support the two forms we currently see from
        // browsers ("permessage-deflate" and
        // "permessage-deflate; client_max_window_bits"), but we still do NOT
        // perform full RFC 7692 parameter negotiation (for example:
        // `server_max_window_bits`, `*_no_context_takeover`, multiple offers,
        // unknown params).
        // Instead, if the client offers permessage-deflate at all, we enable
        // `DeflateConfig::default()` and reply with bare `permessage-deflate`.
        // This is a pragmatic compatibility path, but not fully spec-compliant.
        let client_offers_deflate = parts
            .headers
            .get_all(HeaderName::from_static("sec-websocket-extensions"))
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(|value| value.split(','))
            .map(str::trim)
            .any(|ext| {
                [
                    "permessage-deflate",
                    "permessage-deflate; client_max_window_bits",
                ]
                .contains(&ext)
            });

        Ok(Self {
            config: Default::default(),
            protocol: None,
            sec_websocket_key,
            on_upgrade,
            sec_websocket_protocol,
            on_failed_upgrade: DefaultOnFailedUpgrade,
            client_offers_deflate,
        })
    }
}

fn header_eq(headers: &HeaderMap, key: &HeaderName, value: &'static str) -> bool {
    if let Some(header) = headers.get(key) {
        header.as_bytes().eq_ignore_ascii_case(value.as_bytes())
    } else {
        false
    }
}

fn header_contains(headers: &HeaderMap, key: &HeaderName, value: &'static str) -> bool {
    let Some(header) = headers.get(key) else {
        return false;
    };

    if let Ok(header) = std::str::from_utf8(header.as_bytes()) {
        header.to_ascii_lowercase().contains(value)
    } else {
        false
    }
}

fn trim_ascii(mut bytes: &[u8]) -> &[u8] {
    while let Some((first, rest)) = bytes.split_first() {
        if first.is_ascii_whitespace() {
            bytes = rest;
        } else {
            break;
        }
    }
    while let Some((last, rest)) = bytes.split_last() {
        if last.is_ascii_whitespace() {
            bytes = rest;
        } else {
            break;
        }
    }
    bytes
}

#[derive(Debug)]
pub struct WebSocket {
    inner: WebSocketStream<TokioIo<hyper::upgrade::Upgraded>>,
    protocol: Option<HeaderValue>,
}

impl WebSocket {
    pub async fn recv(&mut self) -> Option<Result<Message, Error>> {
        self.next().await
    }

    pub async fn send(&mut self, msg: Message) -> Result<(), Error> {
        self.inner
            .send(msg.into_tungstenite())
            .await
            .map_err(Error::new)
    }

    pub fn protocol(&self) -> Option<&HeaderValue> {
        self.protocol.as_ref()
    }
}

impl FusedStream for WebSocket {
    fn is_terminated(&self) -> bool {
        self.inner.is_terminated()
    }
}

impl Stream for WebSocket {
    type Item = Result<Message, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match ready!(self.inner.poll_next_unpin(cx)) {
                Some(Ok(msg)) => {
                    if let Some(msg) = Message::from_tungstenite(msg) {
                        return Poll::Ready(Some(Ok(msg)));
                    }
                },
                Some(Err(err)) => return Poll::Ready(Some(Err(Error::new(err)))),
                None => return Poll::Ready(None),
            }
        }
    }
}

impl Sink<Message> for WebSocket {
    type Error = Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.inner).poll_ready(cx).map_err(Error::new)
    }

    fn start_send(mut self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        Pin::new(&mut self.inner)
            .start_send(item.into_tungstenite())
            .map_err(Error::new)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.inner).poll_flush(cx).map_err(Error::new)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.inner).poll_close(cx).map_err(Error::new)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Utf8Bytes(ts::Utf8Bytes);

impl Utf8Bytes {
    #[inline]
    #[must_use]
    pub const fn from_static(str: &'static str) -> Self {
        Self(ts::Utf8Bytes::from_static(str))
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    fn into_tungstenite(self) -> ts::Utf8Bytes {
        self.0
    }
}

impl std::ops::Deref for Utf8Bytes {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl std::fmt::Display for Utf8Bytes {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<Bytes> for Utf8Bytes {
    type Error = std::str::Utf8Error;

    #[inline]
    fn try_from(bytes: Bytes) -> Result<Self, Self::Error> {
        Ok(Self(bytes.try_into()?))
    }
}

impl TryFrom<Vec<u8>> for Utf8Bytes {
    type Error = std::str::Utf8Error;

    #[inline]
    fn try_from(v: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Self(v.try_into()?))
    }
}

impl From<String> for Utf8Bytes {
    #[inline]
    fn from(s: String) -> Self {
        Self(s.into())
    }
}

impl From<&str> for Utf8Bytes {
    #[inline]
    fn from(s: &str) -> Self {
        Self(s.into())
    }
}

impl From<&String> for Utf8Bytes {
    #[inline]
    fn from(s: &String) -> Self {
        Self(s.into())
    }
}

impl From<Utf8Bytes> for Bytes {
    #[inline]
    fn from(Utf8Bytes(bytes): Utf8Bytes) -> Self {
        bytes.into()
    }
}

impl<T> PartialEq<T> for Utf8Bytes
where
    for<'a> &'a str: PartialEq<T>,
{
    #[inline]
    fn eq(&self, other: &T) -> bool {
        self.as_str() == *other
    }
}

pub type CloseCode = u16;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CloseFrame {
    pub code: CloseCode,
    pub reason: Utf8Bytes,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Message {
    Text(Utf8Bytes),
    Binary(Bytes),
    Ping(Bytes),
    Pong(Bytes),
    Close(Option<CloseFrame>),
}

impl Message {
    fn into_tungstenite(self) -> ts::Message {
        match self {
            Self::Text(text) => ts::Message::Text(text.into_tungstenite()),
            Self::Binary(binary) => ts::Message::Binary(binary),
            Self::Ping(ping) => ts::Message::Ping(ping),
            Self::Pong(pong) => ts::Message::Pong(pong),
            Self::Close(Some(close)) => ts::Message::Close(Some(ts::protocol::CloseFrame {
                code: ts::protocol::frame::coding::CloseCode::from(close.code),
                reason: close.reason.into_tungstenite(),
            })),
            Self::Close(None) => ts::Message::Close(None),
        }
    }

    fn from_tungstenite(message: ts::Message) -> Option<Self> {
        match message {
            ts::Message::Text(text) => Some(Self::Text(Utf8Bytes(text))),
            ts::Message::Binary(binary) => Some(Self::Binary(binary)),
            ts::Message::Ping(ping) => Some(Self::Ping(ping)),
            ts::Message::Pong(pong) => Some(Self::Pong(pong)),
            ts::Message::Close(Some(close)) => Some(Self::Close(Some(CloseFrame {
                code: close.code.into(),
                reason: Utf8Bytes(close.reason),
            }))),
            ts::Message::Close(None) => Some(Self::Close(None)),
            ts::Message::Frame(_) => None,
        }
    }

    pub fn into_data(self) -> Bytes {
        match self {
            Self::Text(string) => Bytes::from(string),
            Self::Binary(data) | Self::Ping(data) | Self::Pong(data) => data,
            Self::Close(None) => Bytes::new(),
            Self::Close(Some(frame)) => Bytes::from(frame.reason),
        }
    }

    pub fn into_text(self) -> Result<Utf8Bytes, Error> {
        match self {
            Self::Text(string) => Ok(string),
            Self::Binary(data) | Self::Ping(data) | Self::Pong(data) => {
                Ok(Utf8Bytes::try_from(data).map_err(Error::new)?)
            },
            Self::Close(None) => Ok(Utf8Bytes::default()),
            Self::Close(Some(frame)) => Ok(frame.reason),
        }
    }

    pub fn to_text(&self) -> Result<&str, Error> {
        match *self {
            Self::Text(ref string) => Ok(string.as_str()),
            Self::Binary(ref data) | Self::Ping(ref data) | Self::Pong(ref data) => {
                Ok(std::str::from_utf8(data).map_err(Error::new)?)
            },
            Self::Close(None) => Ok(""),
            Self::Close(Some(ref frame)) => Ok(&frame.reason),
        }
    }

    pub fn text<S>(string: S) -> Self
    where
        S: Into<Utf8Bytes>,
    {
        Self::Text(string.into())
    }

    pub fn binary<B>(bin: B) -> Self
    where
        B: Into<Bytes>,
    {
        Self::Binary(bin.into())
    }
}

impl From<String> for Message {
    fn from(string: String) -> Self {
        Self::Text(string.into())
    }
}

impl<'s> From<&'s str> for Message {
    fn from(string: &'s str) -> Self {
        Self::Text(string.into())
    }
}

impl<'b> From<&'b [u8]> for Message {
    fn from(data: &'b [u8]) -> Self {
        Self::Binary(Bytes::copy_from_slice(data))
    }
}

impl From<Bytes> for Message {
    fn from(data: Bytes) -> Self {
        Self::Binary(data)
    }
}

impl From<Vec<u8>> for Message {
    fn from(data: Vec<u8>) -> Self {
        Self::Binary(data.into())
    }
}

impl From<Message> for Vec<u8> {
    fn from(msg: Message) -> Self {
        msg.into_data().to_vec()
    }
}

fn sign(key: &[u8]) -> HeaderValue {
    let mut sha1 = Sha1::default();
    sha1.update(key);
    sha1.update(&b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11"[..]);
    // Convex fork: workspace uses base64 0.13 API.
    let b64 = Bytes::from(base64::encode(sha1.finalize()));
    HeaderValue::from_maybe_shared(b64).expect("base64 is a valid value")
}
