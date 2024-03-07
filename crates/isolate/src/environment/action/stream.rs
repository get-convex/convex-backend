use anyhow::Context;
use axum::extract::FromRequest;
use common::runtime::Runtime;
use errors::ErrorMetadata;
use futures::{
    stream::BoxStream,
    StreamExt,
};
use http::Request;

use super::task_executor::TaskExecutor;
use crate::environment::action::task::{
    FormPart,
    FormPartFile,
    TaskResponse,
};

impl<RT: Runtime> TaskExecutor<RT> {
    // Sends a stream to javascript by sending TaskResponse::StreamExtend
    // repeatedly. Any errors are sent with StreamExtend, and the number of bytes
    // sent are returned on success.
    pub async fn send_stream(
        &self,
        stream_id: uuid::Uuid,
        stream: Option<BoxStream<'static, anyhow::Result<bytes::Bytes>>>,
    ) -> Result<usize, ()> {
        let mut size = 0;
        if let Some(mut stream) = stream {
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Err(e) => {
                        _ = self
                            .task_retval_sender
                            .unbounded_send(TaskResponse::StreamExtend {
                                stream_id,
                                chunk: Err(ErrorMetadata::bad_request(
                                    "StreamFailed",
                                    e.to_string(),
                                )
                                .into()),
                            });
                        return Err(());
                    },
                    Ok(chunk) => {
                        size += chunk.len();
                        _ = self
                            .task_retval_sender
                            .unbounded_send(TaskResponse::StreamExtend {
                                stream_id,
                                chunk: Ok(Some(chunk)),
                            });
                    },
                }
            }
        }
        // Successfully sent all chunks.
        _ = self
            .task_retval_sender
            .unbounded_send(TaskResponse::StreamExtend {
                stream_id,
                chunk: Ok(None),
            });
        Ok(size)
    }

    pub async fn run_parse_multi_part(
        &self,
        content_type: String,
        request_stream: BoxStream<'static, anyhow::Result<bytes::Bytes>>,
    ) -> anyhow::Result<Vec<FormPart>> {
        let request = Request::builder()
            .header("Content-Type", content_type)
            .body(axum::body::Body::wrap_stream(request_stream))?;
        let mut multipart = axum::extract::Multipart::from_request(request, &())
            .await
            .map_err(|e| ErrorMetadata::bad_request("InvalidMultiPartForm", e.to_string()))?;
        let mut results = vec![];
        while let Some(field) = multipart.next_field().await? {
            let name = field
                .name()
                .with_context(|| {
                    ErrorMetadata::bad_request(
                        "InvalidMultiPartForm",
                        "multi-part form entry missing 'name'",
                    )
                })?
                .to_string();
            let (file, text) = match field.file_name() {
                None => (None, Some(field.text().await?)),
                Some(file_name) => {
                    let file_name = Some(file_name.to_string());
                    let content_type = field.content_type().map(|c| c.to_string());
                    let data = field.bytes().await?.to_vec().into();
                    (
                        Some(FormPartFile {
                            content_type,
                            file_name,
                            data,
                        }),
                        None,
                    )
                },
            };
            results.push(FormPart { name, text, file });
        }
        Ok(results)
    }
}
