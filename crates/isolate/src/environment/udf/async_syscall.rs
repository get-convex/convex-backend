#![allow(non_snake_case)]

use std::{
    collections::BTreeMap,
    convert::{
        TryFrom,
        TryInto,
    },
};

use anyhow::Context;
use common::{
    document::GenericDocument,
    knobs::MAX_SYSCALL_BATCH_SIZE,
    query::{
        Cursor,
        CursorPosition,
        Query,
    },
    runtime::{
        Runtime,
        RuntimeInstant,
        UnixTimestamp,
    },
    value::ConvexValue,
};
use database::{
    query::{
        CompiledQuery,
        QueryType,
    },
    soft_data_limit,
    DeveloperQuery,
    PatchValue,
    Transaction,
    UserFacingModel,
};
use deno_core::v8;
use errors::{
    ErrorMetadata,
    ErrorMetadataAnyhowExt,
};
use itertools::Itertools;
use model::{
    file_storage::{
        types::FileStorageEntry,
        FileStorageId,
    },
    scheduled_jobs::VirtualSchedulerModel,
};
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::{
    json,
    Value as JsonValue,
};
use value::{
    heap_size::HeapSize,
    id_v6::DocumentIdV6,
    TableName,
};

use super::DatabaseUdfEnvironment;
use crate::{
    environment::{
        helpers::{
            parse_version,
            validation::validate_schedule_args,
            with_argument_error,
            ArgName,
        },
        IsolateEnvironment,
    },
    helpers::UdfArgsJson,
    metrics::async_syscall_timer,
};

pub struct PendingSyscall {
    pub name: String,
    pub args: JsonValue,
    pub resolver: v8::Global<v8::PromiseResolver>,
}

impl HeapSize for PendingSyscall {
    fn heap_size(&self) -> usize {
        self.name.heap_size() + self.args.heap_size()
    }
}

// Checks if the underlying table and the request's expectation for the table
// line up.
fn system_table_guard(name: &TableName, expect_system_table: bool) -> anyhow::Result<()> {
    if expect_system_table && !name.is_system() {
        return Err(anyhow::anyhow!(ErrorMetadata::bad_request(
            "SystemTableError",
            "User tables cannot be accessed with db.system."
        )));
    } else if !expect_system_table && name.is_system() {
        return Err(anyhow::anyhow!(ErrorMetadata::bad_request(
            "SystemTableError",
            "System tables can only be accessed with db.system."
        )));
    }
    Ok(())
}

/// A batch of async syscalls that can run "in parallel", where they actually
/// execute in a batch for determinism, but as far as the js promises are
/// concerned, they're running in parallel.
/// This could conceivably run reads
/// (get/queryStreamNext/queryPage/storageGetUrl) all in a single batch.
/// We could also run inserts, deletes, patches, and replaces in a batch
/// together, disjoint from reads, as long as the affected IDs are disjoint.
/// For now, we only allow batches of `db.get`s.
/// TODO(lee) implement other kinds of batches.
#[derive(Debug)]
pub enum AsyncSyscallBatch {
    // TODO(lee) consider db.get to be a special case of queryStreamNext so
    // they can be batched together.
    Gets(Vec<JsonValue>),
    QueryStreamNext(Vec<JsonValue>),
    Unbatched { name: String, args: JsonValue },
}

impl AsyncSyscallBatch {
    pub fn new(name: String, args: JsonValue) -> Self {
        match &*name {
            "1.0/get" => Self::Gets(vec![args]),
            "1.0/queryStreamNext" => Self::QueryStreamNext(vec![args]),
            _ => Self::Unbatched { name, args },
        }
    }

    pub fn can_push(&self, name: &str, _args: &JsonValue) -> bool {
        if self.len() >= *MAX_SYSCALL_BATCH_SIZE {
            return false;
        }
        match (self, name) {
            (Self::Gets(_), "1.0/get") => true,
            (Self::Gets(_), _) => false,
            (Self::QueryStreamNext(_), "1.0/queryStreamNext") => true,
            (Self::QueryStreamNext(_), _) => false,
            (Self::Unbatched { .. }, _) => false,
        }
    }

    pub fn push(&mut self, name: String, args: JsonValue) -> anyhow::Result<()> {
        match (&mut *self, &*name) {
            (Self::Gets(batch_args), "1.0/get") => batch_args.push(args),
            (Self::QueryStreamNext(batch_args), "1.0/queryStreamNext") => batch_args.push(args),
            _ => anyhow::bail!("cannot push {name} onto {self:?}"),
        }
        Ok(())
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Gets(_) => "1.0/get",
            Self::QueryStreamNext(_) => "1.0/queryStreamNext",
            Self::Unbatched { name, .. } => name,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Gets(args) => args.len(),
            Self::QueryStreamNext(args) => args.len(),
            Self::Unbatched { .. } => 1,
        }
    }
}

impl<RT: Runtime> DatabaseUdfEnvironment<RT> {
    /// Runs a batch of syscalls, each of which can succeed or fail
    /// independently. The returned vec is the same length as the batch.
    pub async fn run_async_syscall_batch(
        &mut self,
        batch: AsyncSyscallBatch,
    ) -> anyhow::Result<Vec<anyhow::Result<String>>> {
        let start = self.phase.rt.monotonic_now();
        let batch_name = batch.name().to_string();
        let timer = async_syscall_timer(&batch_name);
        // Outer error is a system error that encompases the whole batch, while
        // inner errors are for individual batch items that may be system or developer
        // errors.
        let results = match batch {
            AsyncSyscallBatch::Gets(get_batch_args) => {
                DatabaseSyscallsV1::get_batch(self, get_batch_args).await
            },
            AsyncSyscallBatch::QueryStreamNext(batch_args) => {
                DatabaseSyscallsV1::queryStreamNext_batch(self, batch_args).await
            },
            AsyncSyscallBatch::Unbatched { name, args } => {
                let result = match &name[..] {
                    // Database
                    "1.0/count" => self.async_syscall_count(args).await,
                    "1.0/insert" => DatabaseSyscallsV1::insert(self, args).await,
                    "1.0/shallowMerge" => DatabaseSyscallsV1::shallow_merge(self, args).await,
                    "1.0/replace" => DatabaseSyscallsV1::replace(self, args).await,
                    "1.0/remove" => DatabaseSyscallsV1::remove(self, args).await,
                    "1.0/queryPage" => DatabaseSyscallsV1::queryPage(self, args).await,
                    // Auth
                    "1.0/getUserIdentity" => self.async_syscall_getUserIdentity(args).await,
                    // Storage
                    "1.0/storageDelete" => self.async_syscall_storageDelete(args).await,
                    "1.0/storageGetMetadata" => self.async_syscall_storageGetMetadata(args).await,
                    "1.0/storageGenerateUploadUrl" => {
                        self.async_syscall_storageGenerateUploadUrl(args).await
                    },
                    "1.0/storageGetUrl" => self.async_syscall_storageGetUrl(args).await,
                    // Scheduling
                    "1.0/schedule" => self.async_syscall_schedule(args).await,
                    "1.0/cancel_job" => self.async_syscall_cancel_job(args).await,
                    #[cfg(test)]
                    "slowSyscall" => {
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        Ok(JsonValue::Number(1017.into()))
                    },
                    #[cfg(test)]
                    "reallySlowSyscall" => {
                        std::thread::sleep(std::time::Duration::from_secs(3));
                        Ok(JsonValue::Number(1017.into()))
                    },
                    _ => Err(ErrorMetadata::bad_request(
                        "UnknownAsyncOperation",
                        format!("Unknown async operation {name}"),
                    )
                    .into()),
                };
                vec![result]
            },
        };
        self.syscall_trace.log_async_syscall(
            batch_name,
            start.elapsed(),
            results.iter().all(|result| result.is_ok()),
        );
        timer.finish();
        Ok(results
            .into_iter()
            .map(|result| anyhow::Ok(serde_json::to_string(&result?)?))
            .collect())
    }

    #[convex_macro::instrument_future]
    async fn async_syscall_count(&mut self, args: JsonValue) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct CountArgs {
            table: String,
        }
        let table = with_argument_error("db.count", || {
            let args: CountArgs = serde_json::from_value(args)?;
            args.table.parse().context(ArgName("table"))
        })?;
        let tx = self.phase.tx()?;
        let result = tx.count(&table).await?;

        // Trim to u32 and check for overflow.
        let result = u32::try_from(result)?;
        // Return as f64, which converts to number type in Javascript.
        let result = f64::from(result);
        Ok(ConvexValue::try_from(result)?.into())
    }

    #[convex_macro::instrument_future]
    async fn async_syscall_getUserIdentity(
        &mut self,
        _args: JsonValue,
    ) -> anyhow::Result<JsonValue> {
        // TODO: Somehow make the Transaction aware of the dependency on the user.
        let tx = self.phase.tx()?;
        let user_identity = tx.user_identity();
        if let Some(user_identity) = user_identity {
            return user_identity.try_into();
        }

        Ok(JsonValue::Null)
    }

    #[convex_macro::instrument_future]
    async fn async_syscall_storageGenerateUploadUrl(
        &mut self,
        _args: JsonValue,
    ) -> anyhow::Result<JsonValue> {
        let issued_ts = self.unix_timestamp()?;
        let postUrl = self
            .file_storage
            .generate_upload_url(&self.key_broker, issued_ts)?;
        Ok(serde_json::to_value(postUrl)?)
    }

    #[convex_macro::instrument_future]
    async fn async_syscall_storageGetUrl(&mut self, args: JsonValue) -> anyhow::Result<JsonValue> {
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
            .file_storage
            .get_url(self.phase.tx()?, storage_id)
            .await?;
        Ok(url.into())
    }

    #[convex_macro::instrument_future]
    async fn async_syscall_storageDelete(&mut self, args: JsonValue) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct StorageDeleteArgs {
            storage_id: String,
        }
        let storage_id: FileStorageId = with_argument_error("storage.delete", || {
            let StorageDeleteArgs { storage_id } = serde_json::from_value(args)?;
            storage_id.parse().context(ArgName("storageId"))
        })?;

        // Synchronously delete the file from storage
        self.file_storage
            .delete(self.phase.tx()?, storage_id.clone())
            .await?;

        Ok(JsonValue::Null)
    }

    #[convex_macro::instrument_future]
    async fn async_syscall_storageGetMetadata(
        &mut self,
        args: JsonValue,
    ) -> anyhow::Result<JsonValue> {
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
            .file_storage
            .get_file_entry(self.phase.tx()?, storage_id)
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

    #[convex_macro::instrument_future]
    async fn async_syscall_schedule(&mut self, args: JsonValue) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ScheduleArgs {
            name: String,
            ts: f64,
            args: UdfArgsJson,
        }

        let ScheduleArgs { name, ts, args }: ScheduleArgs =
            with_argument_error("scheduler", || Ok(serde_json::from_value(args)?))?;
        let udf_path = with_argument_error("scheduler", || name.parse().context(ArgName("name")))?;

        let unix_timestamp = self.unix_timestamp()?;
        let tx = self.phase.tx()?;
        let scheduled_ts = UnixTimestamp::from_secs_f64(ts);
        let (udf_path, udf_args) = validate_schedule_args(
            udf_path,
            args.into_arg_vec(),
            scheduled_ts,
            unix_timestamp,
            &self.module_loader,
            tx,
        )
        .await?;

        let virtual_id = VirtualSchedulerModel::new(tx)
            .schedule(udf_path, udf_args, scheduled_ts, self.context.clone())
            .await?;

        Ok(JsonValue::from(virtual_id))
    }

    #[convex_macro::instrument_future]
    async fn async_syscall_cancel_job(&mut self, args: JsonValue) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct CancelJobArgs {
            id: String,
        }
        let tx = self.phase.tx()?;

        let virtual_id_v6 = with_argument_error("db.cancel_job", || {
            let args: CancelJobArgs = serde_json::from_value(args)?;
            let id = DocumentIdV6::decode(&args.id).context(ArgName("id"))?;
            Ok(id)
        })?;

        VirtualSchedulerModel::new(tx).cancel(virtual_id_v6).await?;

        Ok(JsonValue::Null)
    }
}

/// These are syscalls that exist on `db` in `convex/server` for npm versions >=
/// 0.16.0. They expect DocumentIdv6 strings (as opposed to ID classes).
///
/// Most of the common logic lives on `Transaction` or `DatabaseSyscallsShared`,
/// and this is mostly just taking care of the argument parsing.
pub struct DatabaseSyscallsV1<RT: Runtime> {
    _rt: RT,
}

impl<RT: Runtime> DatabaseSyscallsV1<RT> {
    #[convex_macro::instrument_future]
    async fn get_batch(
        env: &mut DatabaseUdfEnvironment<RT>,
        batch_args: Vec<JsonValue>,
    ) -> Vec<anyhow::Result<JsonValue>> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct GetArgs {
            id: String,
            #[serde(default)]
            is_system: bool,
            #[serde(default)]
            version: Option<String>,
        }
        let table_filter = env.table_filter();
        let tx = match env.phase.tx() {
            Ok(tx) => tx,
            Err(e) => {
                return batch_args
                    .iter()
                    .map(|_| Err(e.clone().into()))
                    .collect_vec();
            },
        };

        let mut ids_to_fetch = BTreeMap::new();
        let mut precomputed_results = BTreeMap::new();
        let batch_size = batch_args.len();
        for (idx, args) in batch_args.into_iter().enumerate() {
            let result: anyhow::Result<_> = try {
                let (id, is_system, version) = with_argument_error("db.get", || {
                    let args: GetArgs = serde_json::from_value(args)?;
                    let id = DocumentIdV6::decode(&args.id).context(ArgName("id"))?;
                    let version = parse_version(args.version)?;
                    Ok((id, args.is_system, version))
                })?;
                let name = tx.all_tables_number_to_name(table_filter)(*id.table());
                if name.is_ok() {
                    system_table_guard(&name?, is_system)?;
                }
                match tx.resolve_idv6(id, table_filter) {
                    Ok(_) => {
                        ids_to_fetch.insert(idx, (id, version));
                    },
                    Err(_) => {
                        // Get on a non-existent table should return null
                        assert!(precomputed_results
                            .insert(idx, Ok(JsonValue::Null))
                            .is_none());
                    },
                }
            };
            if let Err(e) = result {
                assert!(precomputed_results.insert(idx, Err(e)).is_none());
            }
        }

        let mut fetched_results = UserFacingModel::new(tx).get_batch(ids_to_fetch).await;
        (0..batch_size)
            .map(|batch_key| {
                if let Some(precomputed) = precomputed_results.remove(&batch_key) {
                    precomputed
                } else if let Some(fetched_result) = fetched_results.remove(&batch_key) {
                    match fetched_result {
                        Err(e) => Err(e),
                        Ok(Some((doc, _))) => Ok(ConvexValue::Object(doc.into_value().0).into()),
                        // Document does not exist.
                        Ok(None) => Ok(JsonValue::Null),
                    }
                } else {
                    Err(anyhow::anyhow!("missing batch_key {batch_key}"))
                }
            })
            .collect()
    }

    #[convex_macro::instrument_future]
    async fn insert(
        env: &mut DatabaseUdfEnvironment<RT>,
        args: JsonValue,
    ) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct InsertArgs {
            table: String,
            value: JsonValue,
        }
        let (table, value) = with_argument_error("db.insert", || {
            let args: InsertArgs = serde_json::from_value(args)?;
            Ok((
                args.table.parse().context(ArgName("table"))?,
                ConvexValue::try_from(args.value)
                    .context(ArgName("value"))?
                    .try_into()
                    .context(ArgName("value"))?,
            ))
        })?;

        system_table_guard(&table, false)?;
        let tx = env.phase.tx()?;
        let document_id = UserFacingModel::new(tx).insert(table, value).await?;
        let id_str = document_id.encode();
        Ok(json!({ "_id": id_str }))
    }

    #[convex_macro::instrument_future]
    async fn shallow_merge(
        env: &mut DatabaseUdfEnvironment<RT>,
        args: JsonValue,
    ) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct UpdateArgs {
            id: String,
            value: JsonValue,
        }
        let table_filter = env.table_filter();
        let tx = env.phase.tx()?;
        let (id, value, table_name) = with_argument_error("db.patch", || {
            let args: UpdateArgs = serde_json::from_value(args)?;

            let id = DocumentIdV6::decode(&args.id).context(ArgName("id"))?;
            let table_name = tx.resolve_idv6(id, table_filter).context(ArgName("id"))?;

            let value = PatchValue::try_from(args.value).context(ArgName("value"))?;
            Ok((id, value, table_name))
        })?;

        system_table_guard(&table_name, false)?;

        let document = UserFacingModel::new(tx).patch(id, value).await?;
        Ok(document.into_value().0.into())
    }

    #[convex_macro::instrument_future]
    async fn replace(
        env: &mut DatabaseUdfEnvironment<RT>,
        args: JsonValue,
    ) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ReplaceArgs {
            id: String,
            value: JsonValue,
        }
        let table_filter = env.table_filter();
        let tx = env.phase.tx()?;
        let (id, value, table_name) = with_argument_error("db.replace", || {
            let args: ReplaceArgs = serde_json::from_value(args)?;

            let id = DocumentIdV6::decode(&args.id).context(ArgName("id"))?;
            let table_name = tx.resolve_idv6(id, table_filter).context(ArgName("id"))?;

            let value = ConvexValue::try_from(args.value).context(ArgName("value"))?;
            Ok((id, value.try_into().context(ArgName("value"))?, table_name))
        })?;

        system_table_guard(&table_name, false)?;

        let document = UserFacingModel::new(tx).replace(id, value).await?;
        Ok(document.into_value().0.into())
    }

    #[convex_macro::instrument_future]
    async fn queryStreamNext_batch(
        env: &mut DatabaseUdfEnvironment<RT>,
        batch_args: Vec<JsonValue>,
    ) -> Vec<anyhow::Result<JsonValue>> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct QueryStreamNextArgs {
            // not dead code, clippy is wrong.
            #[allow(dead_code)]
            query_id: u32,
        }

        let mut queries_to_fetch = BTreeMap::new();
        let mut results = BTreeMap::new();
        let batch_size = batch_args.len();
        for (idx, args) in batch_args.into_iter().enumerate() {
            let result: anyhow::Result<_> =
                try {
                    let query_id = with_argument_error("queryStreamNext", || {
                        let args: QueryStreamNextArgs = serde_json::from_value(args)?;
                        Ok(args.query_id)
                    })?;
                    let local_query = env.developer_queries.remove(&query_id).context(
                        ErrorMetadata::not_found("QueryNotFound", "in-progress query not found"),
                    )?;
                    queries_to_fetch.insert(idx, (query_id, local_query));
                };
            if let Err(e) = result {
                assert!(results.insert(idx, Err(e)).is_none());
            }
        }

        let tx = match env.phase.tx() {
            Ok(tx) => tx,
            Err(e) => {
                return (0..batch_size).map(|_| Err(e.clone().into())).collect_vec();
            },
        };

        // TODO(lee) actually batch this fetch. For now we do faux-batching to
        // establish the interface.
        let mut fetch_results = BTreeMap::new();
        for (batch_key, (_, local_query)) in queries_to_fetch.iter_mut() {
            fetch_results.insert(*batch_key, local_query.next(tx, None).await);
        }

        #[derive(Serialize)]
        struct QueryStreamNextResult {
            value: JsonValue,
            done: bool,
        }

        for (batch_key, (query_id, local_query)) in queries_to_fetch {
            env.developer_queries.insert(query_id, local_query);

            let result: anyhow::Result<_> = try {
                let maybe_next = fetch_results
                    .remove(&batch_key)
                    .context("batch_key missing")??;

                let done = maybe_next.is_none();
                let value = match maybe_next {
                    Some(doc) => doc.into_value().0.into(),
                    None => ConvexValue::Null,
                };

                if done {
                    env.cleanup_developer_query(query_id);
                }
                serde_json::to_value(QueryStreamNextResult {
                    value: value.into(),
                    done,
                })?
            };
            results.insert(batch_key, result);
        }
        assert_eq!(results.len(), batch_size);
        results.into_values().collect()
    }

    #[convex_macro::instrument_future]
    async fn queryPage(
        env: &mut DatabaseUdfEnvironment<RT>,
        args: JsonValue,
    ) -> anyhow::Result<JsonValue> {
        DatabaseSyscallsShared::queryPage(env, args).await
    }

    #[convex_macro::instrument_future]
    async fn remove(
        env: &mut DatabaseUdfEnvironment<RT>,
        args: JsonValue,
    ) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RemoveArgs {
            id: String,
        }

        let table_filter = env.table_filter();
        let tx = env.phase.tx()?;
        let (id, table_name) = with_argument_error("db.delete", || {
            let args: RemoveArgs = serde_json::from_value(args)?;
            let id = DocumentIdV6::decode(&args.id).context(ArgName("id"))?;
            let table_name = tx.resolve_idv6(id, table_filter).context(ArgName("id"))?;
            Ok((id, table_name))
        })?;

        system_table_guard(&table_name, false)?;

        let document = UserFacingModel::new(tx).delete(id).await?;
        Ok(document.into_value().0.into())
    }
}

struct DatabaseSyscallsShared<RT: Runtime> {
    _rt: RT,
}

/// As pages of results are commonly returned directly from UDFs, a page should
/// be convertible to Value::Array, which has a size limit of MAX_ARRAY_LEN.
/// If there are more results than 75% of that, we recommend splitting the page.
const SOFT_MAX_PAGE_LEN: usize = soft_data_limit(8192);

#[derive(Debug, Copy, Clone)]
enum QueryPageStatus {
    SplitRequired,
    SplitRecommended,
}

impl QueryPageStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SplitRecommended => "SplitRecommended",
            Self::SplitRequired => "SplitRequired",
        }
    }
}

struct QueryPageMetadata {
    cursor: Option<Cursor>,
    split_cursor: Option<Cursor>,
    page_status: Option<QueryPageStatus>,
}

impl<RT: Runtime> DatabaseSyscallsShared<RT> {
    async fn read_page_from_query<T: QueryType>(
        mut query: CompiledQuery<RT, T>,
        tx: &mut Transaction<RT>,
        page_size: usize,
    ) -> anyhow::Result<(Vec<GenericDocument<T::T>>, QueryPageMetadata)> {
        let end_cursor = query.end_cursor();
        let has_end_cursor = end_cursor.is_some();
        let mut page = Vec::with_capacity(page_size);
        let mut page_status = None;
        // If we don't have an end cursor, collect results until we hit our page size.
        // If we do have an end cursor, ignore the page size and collect everything
        while has_end_cursor || (page.len() < page_size) {
            // If we don't have an end cursor, we really have no idea
            // how many results we need to prefetch, but we can
            // use the original page size as a hint.
            let prefetch_hint = if has_end_cursor {
                Some(page_size)
            } else {
                Some(page_size - page.len())
            };

            let next_value = match query.next(tx, prefetch_hint).await {
                Ok(Some(v)) => v,
                Ok(None) => {
                    break;
                },
                Err(e) => {
                    if e.is_pagination_limit() {
                        page_status = Some(QueryPageStatus::SplitRequired);
                        break;
                    }
                    anyhow::bail!(e);
                },
            };
            page.push(next_value)
        }
        if page_status.is_none()
            && (query.is_approaching_data_limit() || page.len() > SOFT_MAX_PAGE_LEN)
        {
            page_status = Some(QueryPageStatus::SplitRecommended);
        }
        Ok((
            page,
            QueryPageMetadata {
                cursor: end_cursor.or_else(|| query.cursor()),
                split_cursor: query.split_cursor(),
                page_status,
            },
        ))
    }

    async fn queryPage(
        env: &mut DatabaseUdfEnvironment<RT>,
        args: JsonValue,
    ) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct QueryPageArgs {
            query: JsonValue,
            cursor: Option<String>,
            end_cursor: Option<String>,
            page_size: usize,
            maximum_rows_read: Option<usize>,
            maximum_bytes_read: Option<usize>,
            #[serde(default)]
            version: Option<String>,
        }
        let args: QueryPageArgs =
            with_argument_error("queryPage", || Ok(serde_json::from_value(args)?))?;
        let parsed_query = with_argument_error("queryPage", || {
            Query::try_from(args.query).context(ArgName("query"))
        })?;
        let version = parse_version(args.version)?;
        let table_filter = env.table_filter();

        let page_size = args.page_size;
        if page_size == 0 {
            anyhow::bail!(ErrorMetadata::bad_request(
                "NoDocumentsForPagination",
                "Must request at least 1 document while paginating"
            ));
        }

        let start_cursor = args
            .cursor
            .map(|c| env.key_broker.decrypt_cursor(c, env.persistence_version))
            .transpose()?;

        let tx = env.phase.tx()?;

        let end_cursor = match args.end_cursor {
            Some(end_cursor) => Some(
                env.key_broker
                    .decrypt_cursor(end_cursor, env.persistence_version)?,
            ),
            None => env.prev_journal.end_cursor.clone(),
        };

        let (
            page,
            QueryPageMetadata {
                cursor,
                split_cursor,
                page_status,
            },
        ) = {
            let query = DeveloperQuery::new_bounded(
                tx,
                parsed_query,
                start_cursor,
                end_cursor,
                args.maximum_rows_read,
                args.maximum_bytes_read,
                true,
                version,
                table_filter,
            )?;
            let (page, metadata) = Self::read_page_from_query(query, tx, page_size).await?;
            let page = page
                .into_iter()
                .map(|doc| ConvexValue::from(doc.into_value().0).into())
                .collect();
            (page, metadata)
        };

        let page_status = page_status.map(|s| s.as_str());

        // Place split_cursor in the middle.
        let split_cursor = split_cursor.map(|split| {
            env.key_broker
                .encrypt_cursor(&split, env.persistence_version)
        });

        let continue_cursor = match &cursor {
            None => anyhow::bail!(
                "Cursor was None. This should be impossible if `.next` was called on the query."
            ),
            Some(cursor) => env
                .key_broker
                .encrypt_cursor(cursor, env.persistence_version),
        };

        let is_done = matches!(
            cursor,
            Some(Cursor {
                position: CursorPosition::End,
                ..
            })
        );

        anyhow::ensure!(
            env.next_journal.end_cursor.is_none(),
            ErrorMetadata::bad_request(
                "MultiplePaginatedDatabaseQueries",
                "This query or mutation function ran multiple paginated queries. Convex only \
                 supports a single paginated query in each function.",
            )
        );
        env.next_journal.end_cursor = cursor;

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct QueryPageResult {
            page: Vec<JsonValue>,
            is_done: bool,
            continue_cursor: String,
            split_cursor: Option<String>,
            page_status: Option<&'static str>,
        }
        let result = QueryPageResult {
            page,
            is_done,
            continue_cursor,
            split_cursor,
            page_status,
        };
        Ok(serde_json::to_value(result)?)
    }
}
