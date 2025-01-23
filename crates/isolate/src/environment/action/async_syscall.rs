#![allow(non_snake_case)]

use anyhow::Context;
use common::{
    bootstrap_model::components::handles::FunctionHandle,
    components::{
        ComponentId,
        Reference,
    },
    runtime::{
        Runtime,
        UnixTimestamp,
    },
};
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use model::{
    components::{
        auth::propagate_component_auth,
        handles::function_handle_not_found,
    },
    file_storage::FileStorageId,
};
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::{
    json,
    Value as JsonValue,
};
use value::id_v6::DeveloperDocumentId;
use vector::{
    VectorSearchJson,
    VectorSearchRequest,
};

use super::task_executor::TaskExecutor;
use crate::{
    environment::helpers::{
        with_argument_error,
        ArgName,
    },
    helpers::UdfArgsJson,
    metrics::async_syscall_timer,
};

impl<RT: Runtime> TaskExecutor<RT> {
    pub fn component_id(&self) -> ComponentId {
        self.component_id
    }

    #[fastrace::trace]
    pub async fn run_async_syscall(&self, name: String, args: JsonValue) -> anyhow::Result<String> {
        let start = self.rt.monotonic_now();
        let timer = async_syscall_timer(&name);
        let result: anyhow::Result<_> = try {
            match &name[..] {
                "1.0/actions/query" => self.async_syscall_actions_runQuery(args).await?,
                "1.0/actions/mutation" => self.async_syscall_actions_runMutation(args).await?,
                "1.0/actions/action" => self.async_syscall_actions_runAction(args).await?,
                "1.0/actions/schedule" => self.async_syscall_schedule(args).await?,
                "1.0/actions/cancel_job" => self.async_syscall_cancel_job(args).await?,
                "1.0/actions/vectorSearch" => self.async_syscall_vectorSearch(args).await?,
                "1.0/getUserIdentity" => self.async_syscall_getUserIdentity(args).await?,
                "1.0/storageDelete" => self.async_syscall_storageDelete(args).await?,
                "1.0/storageGetMetadata" => self.async_syscall_storageGetMetadata(args).await?,
                "1.0/storageGenerateUploadUrl" => {
                    self.async_syscall_storageGenerateUploadUrl(args).await?
                },
                "1.0/storageGetUrl" => self.async_syscall_storageGetUrl(args).await?,
                "1.0/createFunctionHandle" => self.async_syscall_createFunctionHandle(args).await?,
                _ => {
                    anyhow::bail!(ErrorMetadata::bad_request(
                        "UnknownAsyncOperation",
                        format!("Unknown async operation {name}")
                    ));
                },
            }
        };
        self.syscall_trace
            .lock()
            .log_async_syscall(name, start.elapsed(), result.is_ok());
        match &result {
            Ok(_) => timer.finish(),
            Err(e) => timer.finish_with(e.metric_status_label_value()),
        };
        result.and_then(|v| anyhow::Ok(serde_json::to_string(&v)?))
    }

    #[convex_macro::instrument_future]
    async fn async_syscall_actions_runQuery(&self, args: JsonValue) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RunQueryArgs {
            name: Option<String>,
            reference: Option<String>,
            function_handle: Option<String>,
            args: UdfArgsJson,
        }
        let RunQueryArgs {
            name,
            reference,
            function_handle,
            args,
        } = with_argument_error("runQuery", || Ok(serde_json::from_value(args)?))?;
        let function_path = match function_handle {
            Some(function_handle) => {
                let handle: FunctionHandle =
                    with_argument_error("runQuery", || function_handle.parse())?;
                self.action_callbacks
                    .lookup_function_handle(self.identity.clone(), handle)
                    .await?
            },
            None => {
                let reference = parse_name_or_reference("runQuery", name, reference)?;
                self.resolve_function(&reference)?
            },
        };
        let value = self
            .action_callbacks
            .execute_query(
                propagate_component_auth(
                    &self.identity,
                    self.component_id,
                    function_path.component.is_root(),
                ),
                function_path,
                args.into_arg_vec(),
                self.context.clone(),
            )
            .await?
            .result?;
        Ok(JsonValue::from(value))
    }

    #[convex_macro::instrument_future]
    async fn async_syscall_actions_runMutation(
        &self,
        args: JsonValue,
    ) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RunMutationArgs {
            name: Option<String>,
            reference: Option<String>,
            function_handle: Option<String>,
            args: UdfArgsJson,
        }
        let RunMutationArgs {
            name,
            reference,
            function_handle,
            args,
        } = with_argument_error("runMutation", || Ok(serde_json::from_value(args)?))?;
        let function_path = match function_handle {
            Some(function_handle) => {
                let handle: FunctionHandle =
                    with_argument_error("runMutation", || function_handle.parse())?;
                self.action_callbacks
                    .lookup_function_handle(self.identity.clone(), handle)
                    .await?
            },
            None => {
                let reference = parse_name_or_reference("runMutation", name, reference)?;
                self.resolve_function(&reference)?
            },
        };
        let value = self
            .action_callbacks
            .execute_mutation(
                propagate_component_auth(
                    &self.identity,
                    self.component_id,
                    function_path.component.is_root(),
                ),
                function_path,
                args.into_arg_vec(),
                self.context.clone(),
            )
            .await?
            .result?;
        Ok(JsonValue::from(value))
    }

    #[convex_macro::instrument_future]
    async fn async_syscall_actions_runAction(&self, args: JsonValue) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RunActionArgs {
            name: Option<String>,
            reference: Option<String>,
            function_handle: Option<String>,
            args: UdfArgsJson,
        }
        let RunActionArgs {
            name,
            reference,
            function_handle,
            args,
        } = with_argument_error("runAction", || Ok(serde_json::from_value(args)?))?;
        let function_path = match function_handle {
            Some(function_handle) => {
                let handle: FunctionHandle =
                    with_argument_error("runAction", || function_handle.parse())?;
                self.action_callbacks
                    .lookup_function_handle(self.identity.clone(), handle)
                    .await?
            },
            None => {
                let reference = parse_name_or_reference("runAction", name, reference)?;
                self.resolve_function(&reference)?
            },
        };
        let value = self
            .action_callbacks
            .execute_action(
                propagate_component_auth(
                    &self.identity,
                    self.component_id,
                    function_path.component.is_root(),
                ),
                function_path,
                args.into_arg_vec(),
                self.context.clone(),
            )
            .await?
            .result?;
        Ok(JsonValue::from(value))
    }

    #[convex_macro::instrument_future]
    async fn async_syscall_schedule(&self, args: JsonValue) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ScheduleArgs {
            name: Option<String>,
            reference: Option<String>,
            function_handle: Option<String>,
            ts: f64,
            args: UdfArgsJson,
        }

        let ScheduleArgs {
            name,
            reference,
            function_handle,
            ts,
            args,
        }: ScheduleArgs = with_argument_error("scheduler", || Ok(serde_json::from_value(args)?))?;
        let path = match function_handle {
            Some(h) => {
                let handle: FunctionHandle = with_argument_error("scheduler", || h.parse())?;
                self.action_callbacks
                    .lookup_function_handle(self.identity.clone(), handle)
                    .await?
            },
            None => {
                let reference = parse_name_or_reference("scheduler", name, reference)?;
                self.resolve_function(&reference)?
            },
        };
        let scheduled_ts = UnixTimestamp::from_secs_f64(ts);
        let virtual_id = self
            .action_callbacks
            .schedule_job(
                self.identity.clone(),
                self.component_id(),
                path,
                args.into_arg_vec(),
                scheduled_ts,
                self.context.clone(),
            )
            .await?;

        Ok(JsonValue::from(virtual_id))
    }

    #[convex_macro::instrument_future]
    async fn async_syscall_cancel_job(&self, args: JsonValue) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct CancelJobArgs {
            id: String,
        }
        let virtual_id_v6 = with_argument_error("db.cancel_job", || {
            let args: CancelJobArgs = serde_json::from_value(args)?;
            let id = DeveloperDocumentId::decode(&args.id).context(ArgName("id"))?;
            Ok(id)
        })?;

        self.action_callbacks
            .cancel_job(self.identity.clone(), virtual_id_v6)
            .await?;
        Ok(JsonValue::Null)
    }

    #[convex_macro::instrument_future]
    async fn async_syscall_vectorSearch(&self, args: JsonValue) -> anyhow::Result<JsonValue> {
        let VectorSearchRequest { query } = serde_json::from_value(args)?;
        let component_id = self.component_id();
        let mut vector_search_query: VectorSearchJson = serde_json::from_value(query)?;
        vector_search_query.insert_component_id(component_id);

        let (results, usage_stats) = self
            .action_callbacks
            .vector_search(
                self.identity.clone(),
                serde_json::to_value(vector_search_query)?,
            )
            .await?;
        self.usage_tracker.add(usage_stats);
        let results: Vec<_> = results.into_iter().map(JsonValue::from).collect();
        Ok(json!({ "results": results }))
    }

    #[convex_macro::instrument_future]
    async fn async_syscall_getUserIdentity(&self, _args: JsonValue) -> anyhow::Result<JsonValue> {
        self.user_identity()
    }

    async fn async_syscall_storageGenerateUploadUrl(
        &self,
        _args: JsonValue,
    ) -> anyhow::Result<JsonValue> {
        let issued_ts = self.rt.unix_timestamp();
        let component = self.component_id();
        let postUrl =
            self.file_storage
                .generate_upload_url(&self.key_broker, issued_ts, component)?;
        Ok(serde_json::to_value(postUrl)?)
    }

    #[convex_macro::instrument_future]
    async fn async_syscall_storageGetUrl(&self, args: JsonValue) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct GetUrlArgs {
            storage_id: String,
        }
        let storage_id: FileStorageId = with_argument_error("storage.getUrl", || {
            let GetUrlArgs { storage_id } = serde_json::from_value(args)?;
            storage_id.parse().context(ArgName("storageId"))
        })?;
        let url = self
            .action_callbacks
            .storage_get_url(self.identity.clone(), self.component_id(), storage_id)
            .await?;
        Ok(url.into())
    }

    #[convex_macro::instrument_future]
    async fn async_syscall_storageDelete(&self, args: JsonValue) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct StorageDeleteArgs {
            storage_id: String,
        }
        let storage_id: FileStorageId = with_argument_error("storage.delete", || {
            let StorageDeleteArgs { storage_id } = serde_json::from_value(args)?;
            storage_id.parse().context(ArgName("storageId"))
        })?;

        self.action_callbacks
            .storage_delete(self.identity.clone(), self.component_id(), storage_id)
            .await?;

        Ok(JsonValue::Null)
    }

    #[convex_macro::instrument_future]
    async fn async_syscall_storageGetMetadata(&self, args: JsonValue) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct StorageGetMetadataArgs {
            storage_id: String,
        }
        let storage_id: FileStorageId = with_argument_error("storage.getMetadata", || {
            let StorageGetMetadataArgs { storage_id } = serde_json::from_value(args)?;
            storage_id.parse().context(ArgName("storageId"))
        })?;

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct FileMetadataJson {
            storage_id: String,
            sha256: String,
            size: i64,
            content_type: Option<String>,
        }
        let file_metadata = self
            .action_callbacks
            .storage_get_file_entry(self.identity.clone(), self.component_id(), storage_id)
            .await?
            .map(|(_, entry)| {
                // NB: `storage_key is an internal field that we shouldn't return in syscalls.
                FileMetadataJson {
                    storage_id: entry.storage_id.to_string(),
                    // TODO(CX-5533) use base64 for consistency.
                    sha256: entry.sha256.as_hex(),
                    size: entry.size,
                    content_type: entry.content_type,
                }
            });
        Ok(serde_json::to_value(file_metadata)?)
    }

    async fn async_syscall_createFunctionHandle(
        &self,
        args: JsonValue,
    ) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct CreateFunctionHandleArgs {
            name: Option<String>,
            function_handle: Option<String>,
            reference: Option<String>,
        }
        let CreateFunctionHandleArgs {
            name,
            function_handle,
            reference,
        } = with_argument_error("createFunctionHandle", || Ok(serde_json::from_value(args)?))?;
        let function_path = match function_handle {
            Some(function_handle) => {
                return Ok(serde_json::to_value(function_handle)?);
            },
            None => {
                let reference = parse_name_or_reference("createFunctionHandle", name, reference)?;
                self.resolve_function(&reference)?
            },
        };
        // TODO(lee) remove preloaded function handles and call action callback instead,
        // after the callback is deployed to backend & usher.
        let handle = {
            let function_handles = self.function_handles.lock();
            function_handles.get(&function_path).cloned()
        };
        let Some(handle) = handle else {
            anyhow::bail!(function_handle_not_found());
        };
        Ok(serde_json::to_value(String::from(handle))?)
    }
}

pub fn parse_name_or_reference(
    function_name: &str,
    name: Option<String>,
    reference: Option<String>,
) -> anyhow::Result<Reference> {
    with_argument_error(function_name, || match (name, reference) {
        (Some(name), _) => Ok(Reference::Function(name.parse()?)),
        (_, Some(reference)) => Ok(reference.parse()?),
        _ => anyhow::bail!("Missing required argument 'name'"),
    })
}
