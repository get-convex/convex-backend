use axum::headers::{
    Header,
    HeaderName,
    HeaderValue,
};
use http::header::CONTENT_DISPOSITION;

// Takes filename
pub struct ContentDispositionAttachment(pub String);

impl Header for ContentDispositionAttachment {
    fn name() -> &'static HeaderName {
        &CONTENT_DISPOSITION
    }

    fn decode<'i, I>(_values: &mut I) -> Result<Self, axum::headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        unimplemented!()
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        let value = format!("attachment; filename={}", self.0);
        let encoded = HeaderValue::from_str(&value)
            .map_err(|_| axum::headers::Error::invalid())
            .unwrap();
        values.extend(std::iter::once(encoded));
    }
}
