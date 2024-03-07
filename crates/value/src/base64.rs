use ::base64::{
    decode_config,
    encode_config,
    URL_SAFE_NO_PAD,
};

pub fn encode_urlsafe(buf: &[u8]) -> String {
    encode_config(buf, URL_SAFE_NO_PAD)
}

pub fn decode_urlsafe(buf: &str) -> anyhow::Result<Vec<u8>> {
    Ok(decode_config(buf, URL_SAFE_NO_PAD)?)
}
