use serde::{
    Deserialize,
    Serialize,
};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SetExportExpirationRequest {
    pub expiration_ts_ns: u64,
}
