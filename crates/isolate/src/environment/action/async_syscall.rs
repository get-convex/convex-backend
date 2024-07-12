#![allow(non_snake_case)]

use anyhow::Context;
use common::{
    components::{
        ComponentId,
        Reference,
    },
    runtime::{
        Runtime,
        RuntimeInstant,
        UnixTimestamp,
    },
};
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use model::file_storage::{
    types::FileStorageEntry,
    FileStorageId,
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
use vector::VectorSearchRequest;

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
    pub fn component_id(&self) -> anyhow::Result<ComponentId> {
        self.component_id
            .lock()
            .context("component_id was not initialized")
    }

    #[minitrace::trace]
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
            args: UdfArgsJson,
        }

        let (reference, args) = with_argument_error("runQuery", || {
            let RunQueryArgs {
                name,
                reference,
                args,
            } = serde_json::from_value(args)?;
            let reference = parse_name_or_reference(name, reference)?;
            Ok((reference, args))
        })?;
        let function_path = self.resolve_function(&reference)?;
        let value = self
            .action_callbacks
            .execute_query(
                self.identity.clone(),
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
            args: UdfArgsJson,
        }
        let (reference, args) = with_argument_error("runMutation", || {
            let RunMutationArgs {
                name,
                reference,
                args,
            } = serde_json::from_value(args)?;
            let reference = parse_name_or_reference(name, reference)?;
            Ok((reference, args))
        })?;
        let function_path = self.resolve_function(&reference)?;
        let value = self
            .action_callbacks
            .execute_mutation(
                self.identity.clone(),
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
            args: UdfArgsJson,
        }
        let (reference, args) = with_argument_error("runAction", || {
            let RunActionArgs {
                name,
                reference,
                args,
            } = serde_json::from_value(args)?;
            let reference = parse_name_or_reference(name, reference)?;
            Ok((reference, args))
        })?;
        let function_path = self.resolve_function(&reference)?;
        let value = self
            .action_callbacks
            .execute_action(
                self.identity.clone(),
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
            ts: f64,
            args: UdfArgsJson,
        }

        let (reference, ts, args) = with_argument_error("scheduler", || {
            let ScheduleArgs {
                name,
                reference,
                ts,
                args,
            } = serde_json::from_value(args)?;
            let reference = parse_name_or_reference(name, reference)?;
            Ok((reference, ts, args))
        })?;
        let path = self.resolve_function(&reference)?;
        let scheduled_ts = UnixTimestamp::from_secs_f64(ts);
        let virtual_id = self
            .action_callbacks
            .schedule_job(
                self.identity.clone(),
                self.component_id()?,
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
        let (results, usage_stats) = self
            .action_callbacks
            .vector_search(self.identity.clone(), query)
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
        let component = self.component_id()?;
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
            .storage_get_url(self.identity.clone(), self.component_id()?, storage_id)
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
            .storage_delete(self.identity.clone(), self.component_id()?, storage_id)
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
            .storage_get_file_entry(self.identity.clone(), self.component_id()?, storage_id)
            .await?
            .map(
                |FileStorageEntry {
                     storage_id,
                     storage_key: _, // internal field that we shouldn't return in syscalls
                     sha256,
                     size,
                     content_type,
                 }| {
                    FileMetadataJson {
                        storage_id: storage_id.to_string(),
                        // TODO(CX-5533) use base64 for consistency.
                        sha256: sha256.as_hex(),
                        size,
                        content_type,
                    }
                },
            );
        Ok(serde_json::to_value(file_metadata)?)
    }
}

pub fn parse_name_or_reference(
    name: Option<String>,
    reference: Option<String>,
) -> anyhow::Result<Reference> {
    match (name, reference) {
        (Some(name), _) => Ok(Reference::Function(name.parse()?)),
        (_, Some(reference)) => Ok(reference.parse()?),
        _ => anyhow::bail!("Missing required argument 'name'"),
    }
}
