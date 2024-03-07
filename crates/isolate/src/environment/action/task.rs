//! Our action runtime runs "tasks" asynchronously, which either be
//! async syscalls or async ops.

use common::runtime::{
    Runtime,
    UnixTimestamp,
};
use deno_core::{
    serde_v8,
    v8,
    ToJsBuffer,
};
use serde::Serialize;
use serde_json::Value as JsonValue;
use value::id_v6::DocumentIdV6;

use crate::{
    environment::{
        helpers::syscall_error::{
            syscall_description_for_error,
            syscall_name_for_error,
        },
        AsyncOpRequest,
        IsolateEnvironment,
    },
    execution_scope::ExecutionScope,
    http::HttpResponseV8,
};

pub struct TaskRequest {
    pub task_id: TaskId,
    pub variant: TaskRequestEnum,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, derive_more::Display)]
pub struct TaskId(pub usize);

impl TaskId {
    pub fn increment(&mut self) -> Self {
        let task_id = *self;
        self.0 += 1;
        task_id
    }
}

pub enum TaskRequestEnum {
    AsyncSyscall { name: String, args: JsonValue },
    AsyncOp(AsyncOpRequest),
}

impl TaskRequestEnum {
    pub fn to_type(&self) -> TaskType {
        match self {
            TaskRequestEnum::AsyncSyscall { name, .. } => TaskType::Syscall(name.clone()),
            TaskRequestEnum::AsyncOp(AsyncOpRequest::Fetch { .. }) => TaskType::Fetch,
            TaskRequestEnum::AsyncOp(AsyncOpRequest::ParseMultiPart { .. }) => {
                TaskType::ParseMultiPart
            },
            TaskRequestEnum::AsyncOp(AsyncOpRequest::Sleep { .. }) => TaskType::Sleep,
            TaskRequestEnum::AsyncOp(AsyncOpRequest::StorageStore { .. }) => TaskType::StorageStore,
            TaskRequestEnum::AsyncOp(AsyncOpRequest::StorageGet { .. }) => TaskType::StorageGet,
            TaskRequestEnum::AsyncOp(AsyncOpRequest::SendStream { .. }) => TaskType::SendStream,
        }
    }

    pub fn name_for_error(&self) -> &'static str {
        match self {
            TaskRequestEnum::AsyncSyscall { name, .. } => syscall_name_for_error(name),
            TaskRequestEnum::AsyncOp(ref op) => op.name_for_error(),
        }
    }

    pub fn description_for_error(&self) -> String {
        match self {
            TaskRequestEnum::AsyncSyscall { name, .. } => syscall_description_for_error(name),
            TaskRequestEnum::AsyncOp(ref op) => op.description_for_error(),
        }
    }
}

pub enum TaskType {
    Syscall(String),
    Fetch,
    ParseMultiPart,
    Sleep,
    StorageStore,
    StorageGet,
    SendStream,
}

fn syscall_display_name(syscall: &str) -> String {
    syscall
        .replacen("1.0/", "", 1)
        .replacen("actions/", "", 1)
        .replacen("httpEndpoint/", "", 1)
        .replacen("shallowMerge", "patch", 1)
        .replacen("queryPage", "paginate", 1)
}

impl TaskType {
    pub fn name_when_dangling(&self) -> String {
        match self {
            TaskType::Syscall(syscall) => syscall_display_name(syscall),
            TaskType::Fetch => "fetch".to_string(),
            TaskType::ParseMultiPart => "formData".to_string(),
            TaskType::StorageStore => "storage.store".to_string(),
            TaskType::StorageGet => "storage.get".to_string(),
            TaskType::SendStream => "ReadableStream".to_string(),
            // Sleeps cannot actually be dangling, but we handle it just in case.
            TaskType::Sleep => "setTimeout".to_string(),
        }
    }
}

pub enum TaskResponse {
    TaskDone {
        task_id: TaskId,
        variant: anyhow::Result<TaskResponseEnum>,
    },
    StreamExtend {
        stream_id: uuid::Uuid,
        chunk: anyhow::Result<Option<bytes::Bytes>>,
    },
}

#[derive(Debug)]
pub enum TaskResponseEnum {
    Syscall(String),
    Fetch(HttpResponseV8),
    ParseMultiPart(Vec<FormPart>),
    Sleep(UnixTimestamp),
    StorageStore(DocumentIdV6),
    StorageGet(Option<FileResponse>),
}

impl TaskResponseEnum {
    pub fn into_v8<'a, 'b: 'a, RT: Runtime, E: IsolateEnvironment<RT>>(
        self,
        scope: &mut ExecutionScope<'a, 'b, RT, E>,
    ) -> anyhow::Result<v8::Local<'a, v8::Value>> {
        let value_v8 = match self {
            Self::Fetch(response) => serde_v8::to_v8(scope, response)?,
            Self::Syscall(s) => serde_v8::to_v8(scope, s)?,
            Self::ParseMultiPart(parts) => serde_v8::to_v8(scope, parts)?,
            Self::Sleep(_) => serde_v8::to_v8(scope, ())?,
            Self::StorageStore(storage_id) => serde_v8::to_v8(scope, storage_id.to_string())?,
            Self::StorageGet(file_response) => serde_v8::to_v8(scope, file_response)?,
        };
        Ok(value_v8)
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileResponse {
    pub body_stream_id: uuid::Uuid,
    pub content_length: u64,
    pub content_type: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormPart {
    pub name: String,
    pub file: Option<FormPartFile>,
    pub text: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormPartFile {
    pub content_type: Option<String>,
    pub data: ToJsBuffer,
    pub file_name: Option<String>,
}
