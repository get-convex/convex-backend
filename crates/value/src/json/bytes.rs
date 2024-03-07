use std::convert::TryFrom;

use crate::ConvexBytes;

/// Helper functions for encoding `Bytes`s as `String`s.
pub enum JsonBytes {}

impl JsonBytes {
    /// Encode a binary string as a string.
    pub fn encode(bytes: &ConvexBytes) -> String {
        base64::encode(&bytes[..])
    }

    /// Decode a binary string from a string.
    pub fn decode(s: String) -> anyhow::Result<ConvexBytes> {
        ConvexBytes::try_from(base64::decode(s.as_bytes())?)
    }
}
