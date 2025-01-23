use headers::{
    Error as HeaderError,
    Header,
    HeaderName,
};
use http::{
    header::InvalidHeaderValue,
    HeaderValue,
};
pub use value::sha256::{
    SetDigest,
    Sha256,
    Sha256Digest,
};

static DIGEST_HEADER: HeaderName = HeaderName::from_static("digest");

#[derive(Clone)]
pub struct DigestHeader(pub Sha256Digest);

impl TryFrom<DigestHeader> for HeaderValue {
    type Error = InvalidHeaderValue;

    fn try_from(value: DigestHeader) -> Result<Self, Self::Error> {
        format!("sha-256={}", value.0.as_base64()).parse()
    }
}

impl Header for DigestHeader {
    fn name() -> &'static HeaderName {
        &DIGEST_HEADER
    }

    fn decode<'i, I: Iterator<Item = &'i HeaderValue>>(
        values: &mut I,
    ) -> Result<Self, HeaderError> {
        values
            .next()
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("sha-256="))
            .and_then(|v| Sha256Digest::from_base64(v).ok())
            .map(DigestHeader)
            .ok_or_else(HeaderError::invalid)
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        let value: HeaderValue = self.clone().try_into().expect("Must be valid header value");
        values.extend([value]);
    }
}
