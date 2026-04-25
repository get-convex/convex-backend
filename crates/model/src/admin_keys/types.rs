use keybroker::AdminKeyHash;
use serde::{
    Deserialize,
    Serialize,
};
use sync_types::Timestamp;
use value::codegen_convex_serialization;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminKeyMetadata {
    /// 32-byte HMAC-SHA-256 of the normalized admin key.
    pub key_hash: AdminKeyHash,
    pub name: String,
    pub revoked_time: Option<Timestamp>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedAdminKeyMetadata {
    /// Stored as hex to keep Convex values as strings rather than bytes.
    key_hash: String,
    name: String,
    revoked_time: Option<i64>,
}

impl TryFrom<AdminKeyMetadata> for SerializedAdminKeyMetadata {
    type Error = anyhow::Error;

    fn try_from(value: AdminKeyMetadata) -> Result<Self, Self::Error> {
        Ok(Self {
            key_hash: hex::encode(value.key_hash.as_bytes()),
            name: value.name,
            revoked_time: value.revoked_time.map(|t| t.into()),
        })
    }
}

impl TryFrom<SerializedAdminKeyMetadata> for AdminKeyMetadata {
    type Error = anyhow::Error;

    fn try_from(value: SerializedAdminKeyMetadata) -> Result<Self, Self::Error> {
        let bytes = hex::decode(&value.key_hash)?;
        anyhow::ensure!(bytes.len() == 32, "Invalid key_hash length");
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self {
            key_hash: AdminKeyHash(arr),
            name: value.name,
            revoked_time: value.revoked_time.map(Timestamp::try_from).transpose()?,
        })
    }
}

codegen_convex_serialization!(AdminKeyMetadata, SerializedAdminKeyMetadata);
