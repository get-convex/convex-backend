use http::{
    HeaderName,
    HeaderValue,
};

use crate::common::HttpHeader;

impl TryFrom<HttpHeader> for (HeaderName, HeaderValue) {
    type Error = anyhow::Error;

    fn try_from(HttpHeader { key, value }: HttpHeader) -> Result<Self, Self::Error> {
        Ok((HeaderName::try_from(key)?, HeaderValue::from_bytes(&value)?))
    }
}

impl From<(HeaderName, HeaderValue)> for HttpHeader {
    fn from((key, value): (HeaderName, HeaderValue)) -> Self {
        HttpHeader {
            key: key.to_string(),
            value: value.as_bytes().to_owned(),
        }
    }
}
