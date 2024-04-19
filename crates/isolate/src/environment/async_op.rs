use std::fmt;

use common::{
    http::HttpRequestStream,
    runtime::UnixTimestamp,
};
use futures::{
    channel::mpsc::UnboundedReceiver,
    stream::BoxStream,
};

pub enum AsyncOpRequest {
    Fetch {
        request: HttpRequestStream,
        response_body_stream_id: uuid::Uuid,
    },
    ParseMultiPart {
        content_type: String,
        request_stream: BoxStream<'static, anyhow::Result<bytes::Bytes>>,
    },
    Sleep {
        name: String, // setTimeout or setInterval
        until: UnixTimestamp,
    },
    StorageStore {
        body_stream: UnboundedReceiver<anyhow::Result<bytes::Bytes>>,
        content_type: Option<String>,
        content_length: Option<String>,
        digest: Option<String>,
    },
    StorageGet {
        storage_id: String,
        stream_id: uuid::Uuid,
    },
    SendStream {
        stream: Option<BoxStream<'static, anyhow::Result<bytes::Bytes>>>,
        stream_id: uuid::Uuid,
    },
}

impl AsyncOpRequest {
    pub fn name_for_error(&self) -> &'static str {
        match self {
            Self::Fetch { .. } => "Fetch",
            Self::ParseMultiPart { .. } => "FormParse",
            Self::Sleep { .. } => "Sleep",
            Self::StorageStore { .. } | Self::StorageGet { .. } => "Storage",
            Self::SendStream { .. } => "Stream",
        }
    }

    pub fn description_for_error(&self) -> String {
        match self {
            Self::Fetch { .. } => "fetch()".to_string(),
            Self::ParseMultiPart { .. } => "formData()".to_string(),
            Self::Sleep { name, .. } => name.to_string(),
            Self::StorageStore { .. } => "storage.store()".to_string(),
            Self::StorageGet { .. } => "storage.get()".to_string(),
            Self::SendStream { .. } => "stream".to_string(),
        }
    }
}

impl fmt::Debug for AsyncOpRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name_for_error().fmt(f)
    }
}
