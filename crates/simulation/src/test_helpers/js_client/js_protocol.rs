use serde::{
    Deserialize,
    Serialize,
};

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
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddQueryArgs {
    pub udf_path: String,
    pub udf_args_json: String,
}
