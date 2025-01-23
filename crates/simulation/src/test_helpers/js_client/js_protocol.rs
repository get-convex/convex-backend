use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum JsOutgoingMessage {
    #[serde(rename_all = "camelCase")]
    Connect { web_socket_id: u32 },

    #[serde(rename_all = "camelCase")]
    Send { web_socket_id: u32, data: String },

    #[serde(rename_all = "camelCase")]
    Close { web_socket_id: u32 },

    #[serde(rename_all = "camelCase")]
    PersistMutation {
        persist_id: String,
        mutation_info: JsonValue,
    },

    #[serde(rename_all = "camelCase")]
    PersistPages {
        persist_id: String,
        pages: Vec<JsonValue>,
    },

    #[serde(rename_all = "camelCase")]
    MutationDone {
        mutation_id: u32,
        result: MutationResult,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum JsIncomingMessage {
    #[serde(rename_all = "camelCase")]
    Connected { web_socket_id: u32 },

    #[serde(rename_all = "camelCase")]
    Message { web_socket_id: u32, data: String },

    #[serde(rename_all = "camelCase")]
    Closed { web_socket_id: u32 },

    #[serde(rename_all = "camelCase")]
    PersistenceDone {
        persist_id: String,
        error: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum MutationResult {
    #[serde(rename_all = "camelCase")]
    Success { value: JsonValue },

    #[serde(rename_all = "camelCase")]
    Failure { error: String },
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddQueryArgs {
    pub udf_path: String,
    pub udf_args_json: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddSyncQueryArgs {
    pub id: String,
    pub name: String,
    pub udf_args_json: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestSyncMutationArgs {
    pub id: String,
    pub mutation_info_json: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum SyncQueryResult {
    #[serde(rename_all = "camelCase")]
    Loading,

    #[serde(rename_all = "camelCase")]
    Success { value: JsonValue },

    #[serde(rename_all = "camelCase")]
    Error { error: String },
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunMutationArgs {
    pub mutation_id: u32,
    pub udf_path: String,
    pub udf_args_json: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "status")]
pub enum SyncMutationStatus {
    Unresolved,
    Reflected,
    ReflectedLocallyButWaitingForNetwork,
    ReflectedOnNetworkButNotLocally,
}
