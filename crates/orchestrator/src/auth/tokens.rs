//! Token minting and parsing.
//!
//! All orchestrator-issued bearer tokens take the form:
//!
//! ```text
//! pat:<public-id>|<secret>
//! ```
//!
//! `public-id` is a 16-char hex ID stored in `access_tokens.public_id`.
//! `secret` is a 32-byte random secret encoded as base64-url (no padding).
//! Storage holds only `sha256(secret)` and the last 4 chars of secret as
//! `secret_suffix` for UI display.

use anyhow::{
    anyhow,
    bail,
};
use rand::{
    rngs::OsRng,
    TryRngCore,
};
use sha2::{
    Digest,
    Sha256,
};

#[derive(Debug, Clone)]
pub struct TokenSecret {
    pub public_id: String,
    pub secret: String,
}

impl TokenSecret {
    pub fn encoded(&self, prefix: &str) -> String {
        format!("{prefix}:{}|{}", self.public_id, self.secret)
    }
}

pub fn mint_token_secret(public_id: &str) -> TokenSecret {
    let mut bytes = [0u8; 32];
    OsRng
        .try_fill_bytes(&mut bytes)
        .expect("OS rng failed");
    // URL-safe base64 without padding, hex-encoded for portability across the
    // older base64 0.13 in the workspace.
    let secret = hex::encode(bytes);
    TokenSecret {
        public_id: public_id.to_string(),
        secret,
    }
}

pub fn encode_pat(t: &TokenSecret) -> String {
    t.encoded("pat")
}

#[derive(Debug, Clone)]
pub struct ParsedToken<'a> {
    pub prefix: &'a str,
    pub public_id: &'a str,
    pub secret: &'a str,
}

pub fn parse_token(input: &str) -> anyhow::Result<ParsedToken<'_>> {
    let (prefix, rest) = input
        .split_once(':')
        .ok_or_else(|| anyhow!("token missing prefix"))?;
    let (public_id, secret) = rest
        .split_once('|')
        .ok_or_else(|| anyhow!("token missing pipe separator"))?;
    if prefix.is_empty() || public_id.is_empty() || secret.is_empty() {
        bail!("token has empty fields");
    }
    Ok(ParsedToken {
        prefix,
        public_id,
        secret,
    })
}

pub fn sha256_hex(input: &str) -> String {
    let mut h = Sha256::new();
    h.update(input.as_bytes());
    hex::encode(h.finalize())
}

pub fn suffix_of(secret: &str) -> String {
    let n = secret.len();
    if n <= 4 {
        secret.to_string()
    } else {
        secret[n.saturating_sub(4)..].to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid() {
        let t = parse_token("pat:abc|xyz").unwrap();
        assert_eq!(t.prefix, "pat");
        assert_eq!(t.public_id, "abc");
        assert_eq!(t.secret, "xyz");
    }

    #[test]
    fn mint_and_hash_round_trip() {
        let m = mint_token_secret("public");
        assert_eq!(m.public_id, "public");
        let h = sha256_hex(&m.secret);
        assert_eq!(h.len(), 64);
        assert_ne!(h, m.secret);
    }

    #[test]
    fn parse_invalid() {
        assert!(parse_token("no-colon").is_err());
        assert!(parse_token("pat:no-pipe").is_err());
        assert!(parse_token(":empty|secret").is_err());
    }
}
