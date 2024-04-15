use std::{
    collections::BTreeMap,
    convert::{
        TryFrom,
        TryInto,
    },
    marker::PhantomData,
    time::Duration,
};

use anyhow::Context;
use common::{
    document::{
        GenericDocument,
        ID_FIELD_PATH,
    },
    execution_context::ExecutionContext,
    knobs::MAX_SYSCALL_BATCH_SIZE,
    maybe_val,
    query::{
        Cursor,
        CursorPosition,
        IndexRange,
        IndexRangeExpression,
        Order,
        Query,
    },
    query_journal::QueryJournal,
    runtime::{
        Runtime,
        RuntimeInstant,
        UnixTimestamp,
    },
    types::{
        IndexName,
        PersistenceVersion,
    },
    value::ConvexValue,
};
use database::{
    query::{
        query_batch_next,
        CompiledQuery,
        QueryType,
        TableFilter,
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
use keybroker::KeyBroker;
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
use sync_types::UdfPath;
use value::{
    heap_size::HeapSize,
    id_v6::DocumentIdV6,
    ConvexArray,
    TableName,
};

use super::DatabaseUdfEnvironment;
use crate::{
    environment::helpers::{
        parse_version,
        validation::validate_schedule_args,
        with_argument_error,
        ArgName,
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
pub fn system_table_guard(name: &TableName, expect_system_table: bool) -> anyhow::Result<()> {
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
/// For now, we only allow batches of `db.get`s and `db.query`s.
/// TODO(lee) implement other kinds of batches.
#[derive(Debug)]
pub enum AsyncSyscallBatch {
    Reads(Vec<AsyncRead>),
    Unbatched { name: String, args: JsonValue },
}

#[derive(Debug)]
pub enum AsyncRead {
    Get(JsonValue),
    QueryStreamNext(JsonValue),
}

impl AsyncSyscallBatch {
    pub fn new(name: String, args: JsonValue) -> Self {
        match &*name {
            "1.0/get" => Self::Reads(vec![AsyncRead::Get(args)]),
            "1.0/queryStreamNext" => Self::Reads(vec![AsyncRead::QueryStreamNext(args)]),
            _ => Self::Unbatched { name, args },
        }
    }

    pub fn can_push(&self, name: &str, _args: &JsonValue) -> bool {
        if self.len() >= *MAX_SYSCALL_BATCH_SIZE {
            return false;
        }
        match (self, name) {
            (Self::Reads(_), "1.0/get") => true,
            (Self::Reads(_), "1.0/queryStreamNext") => true,
            (Self::Reads(_), _) => false,
            (Self::Unbatched { .. }, _) => false,
        }
    }

    pub fn push(&mut self, name: String, args: JsonValue) -> anyhow::Result<()> {
        match (&mut *self, &*name) {
            (Self::Reads(batch_args), "1.0/get") => batch_args.push(AsyncRead::Get(args)),
            (Self::Reads(batch_args), "1.0/queryStreamNext") => {
                batch_args.push(AsyncRead::QueryStreamNext(args))
            },
            _ => anyhow::bail!("cannot push {name} onto {self:?}"),
        }
        Ok(())
    }

    pub fn name(&self) -> &str {
        match self {
            // 1.0/get is grouped in with 1.0/queryStreamNext.
            Self::Reads(_) => "1.0/queryStreamNext",
            Self::Unbatched { name, .. } => name,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Reads(args) => args.len(),
            Self::Unbatched { .. } => 1,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub struct QueryManager<RT: Runtime> {
    next_id: u32,
    developer_queries: BTreeMap<u32, DeveloperQuery<RT>>,
}

impl<RT: Runtime> QueryManager<RT> {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            developer_queries: BTreeMap::new(),
        }
    }

    pub fn put_developer(&mut self, query: DeveloperQuery<RT>) -> u32 {
        let id = self.next_id;
        self.developer_queries.insert(id, query);
        self.next_id += 1;
        id
    }

    pub fn take_developer(&mut self, id: u32) -> Option<DeveloperQuery<RT>> {
        self.developer_queries.remove(&id)
    }

    pub fn insert_developer(&mut self, id: u32, query: DeveloperQuery<RT>) {
        self.developer_queries.insert(id, query);
    }

    pub fn cleanup_developer(&mut self, id: u32) -> bool {
        self.developer_queries.remove(&id).is_some()
    }
}

// Trait for allowing code reuse between `DatabaseUdfEnvironment` and isolate2.
#[allow(async_fn_in_trait)]
pub trait AsyncSyscallProvider<RT: Runtime> {
    fn rt(&self) -> &RT;
    fn tx(&mut self) -> Result<&mut Transaction<RT>, ErrorMetadata>;
    fn key_broker(&self) -> &KeyBroker;
    fn context(&self) -> &ExecutionContext;

    fn unix_timestamp(&self) -> anyhow::Result<UnixTimestamp>;

    fn persistence_version(&self) -> PersistenceVersion;
    fn table_filter(&self) -> TableFilter;

    fn log_async_syscall(&mut self, name: String, duration: Duration, is_success: bool);

    fn query_manager(&mut self) -> &mut QueryManager<RT>;

    fn prev_journal(&mut self) -> &mut QueryJournal;
    fn next_journal(&mut self) -> &mut QueryJournal;

    async fn validate_schedule_args(
        &mut self,
        udf_path: UdfPath,
        args: Vec<JsonValue>,
        scheduled_ts: UnixTimestamp,
    ) -> anyhow::Result<(UdfPath, ConvexArray)>;

    fn file_storage_generate_upload_url(&self) -> anyhow::Result<String>;
    async fn file_storage_get_url(
        &mut self,
        storage_id: FileStorageId,
    ) -> anyhow::Result<Option<String>>;
    async fn file_storage_delete(&mut self, storage_id: FileStorageId) -> anyhow::Result<()>;
    async fn file_storage_get_entry(
        &mut self,
        storage_id: FileStorageId,
    ) -> anyhow::Result<Option<FileStorageEntry>>;
}

impl<RT: Runtime> AsyncSyscallProvider<RT> for DatabaseUdfEnvironment<RT> {
    fn rt(&self) -> &RT {
        &self.phase.rt
    }

    fn tx(&mut self) -> Result<&mut Transaction<RT>, ErrorMetadata> {
        self.phase.tx()
    }

    fn key_broker(&self) -> &KeyBroker {
        &self.key_broker
    }

    fn context(&self) -> &ExecutionContext {
        &self.context
    }

    fn unix_timestamp(&self) -> anyhow::Result<UnixTimestamp> {
        self.phase.unix_timestamp()
    }

    fn persistence_version(&self) -> PersistenceVersion {
        self.persistence_version
    }

    fn table_filter(&self) -> TableFilter {
        if self.udf_path.is_system() {
            TableFilter::IncludePrivateSystemTables
        } else {
            TableFilter::ExcludePrivateSystemTables
        }
    }

    fn log_async_syscall(&mut self, name: String, duration: Duration, is_success: bool) {
        self.syscall_trace
            .log_async_syscall(name, duration, is_success);
    }

    fn query_manager(&mut self) -> &mut QueryManager<RT> {
        &mut self.query_manager
    }

    fn prev_journal(&mut self) -> &mut QueryJournal {
        &mut self.prev_journal
    }

    fn next_journal(&mut self) -> &mut QueryJournal {
        &mut self.next_journal
    }

    async fn validate_schedule_args(
        &mut self,
        udf_path: UdfPath,
        args: Vec<JsonValue>,
        scheduled_ts: UnixTimestamp,
    ) -> anyhow::Result<(UdfPath, ConvexArray)> {
        validate_schedule_args(
            udf_path,
            args,
            scheduled_ts,
            self.phase.unix_timestamp()?,
            &self.module_loader,
            self.phase.tx()?,
        )
        .await
    }

    fn file_storage_generate_upload_url(&self) -> anyhow::Result<String> {
        let issued_ts = self.phase.unix_timestamp()?;
        let post_url = self
            .file_storage
            .generate_upload_url(&self.key_broker, issued_ts)?;
        Ok(post_url)
    }

    async fn file_storage_get_url(
        &mut self,
        storage_id: FileStorageId,
    ) -> anyhow::Result<Option<String>> {
        self.file_storage
            .get_url(self.phase.tx()?, storage_id)
            .await
    }

    async fn file_storage_delete(&mut self, storage_id: FileStorageId) -> anyhow::Result<()> {
        self.file_storage.delete(self.phase.tx()?, storage_id).await
    }

    async fn file_storage_get_entry(
        &mut self,
        storage_id: FileStorageId,
    ) -> anyhow::Result<Option<FileStorageEntry>> {
        self.file_storage
            .get_file_entry(self.phase.tx()?, storage_id)
            .await
    }
}

/// These are syscalls that exist on `db` in `convex/server` for npm versions >=
/// 0.16.0. They expect DocumentIdv6 strings (as opposed to ID classes).
///
/// Most of the common logic lives on `Transaction` or `DatabaseSyscallsShared`,
/// and this is mostly just taking care of the argument parsing.
pub struct DatabaseSyscallsV1<RT: Runtime, P: AsyncSyscallProvider<RT>> {
    _pd: PhantomData<(RT, P)>,
}

impl<RT: Runtime, P: AsyncSyscallProvider<RT>> DatabaseSyscallsV1<RT, P> {
    /// Runs a batch of syscalls, each of which can succeed or fail
    /// independently. The returned vec is the same length as the batch.
    #[minitrace::trace]
    pub async fn run_async_syscall_batch(
        provider: &mut P,
        batch: AsyncSyscallBatch,
    ) -> anyhow::Result<Vec<anyhow::Result<String>>> {
        let start = provider.rt().monotonic_now();
        let batch_name = batch.name().to_string();
        let timer = async_syscall_timer(&batch_name);
        // Outer error is a system error that encompases the whole batch, while
        // inner errors are for individual batch items that may be system or developer
        // errors.
        let results = match batch {
            AsyncSyscallBatch::Reads(batch_args) => Self::query_batch(provider, batch_args).await,
            AsyncSyscallBatch::Unbatched { name, args } => {
                let result = match &name[..] {
                    // Database
                    "1.0/count" => Self::count(provider, args).await,
                    "1.0/insert" => Self::insert(provider, args).await,
                    "1.0/shallowMerge" => Self::shallow_merge(provider, args).await,
                    "1.0/replace" => Self::replace(provider, args).await,
                    "1.0/remove" => Self::remove(provider, args).await,
                    "1.0/queryPage" => Self::query_page(provider, args).await,
                    // Auth
                    "1.0/getUserIdentity" => Self::get_user_identity(provider, args).await,
                    // Storage
                    "1.0/storageDelete" => Self::storage_delete(provider, args).await,
                    "1.0/storageGetMetadata" => Self::storage_get_metadata(provider, args).await,
                    "1.0/storageGenerateUploadUrl" => {
                        Self::storage_generate_upload_url(provider, args).await
                    },
                    "1.0/storageGetUrl" => Self::storage_get_url(provider, args).await,
                    // Scheduling
                    "1.0/schedule" => Self::schedule(provider, args).await,
                    "1.0/cancel_job" => Self::cancel_job(provider, args).await,
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
        provider.log_async_syscall(
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
    async fn count(provider: &mut P, args: JsonValue) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct CountArgs {
            table: String,
        }
        let table = with_argument_error("db.count", || {
            let args: CountArgs = serde_json::from_value(args)?;
            args.table.parse().context(ArgName("table"))
        })?;
        let tx = provider.tx()?;
        let result = tx.count(&table).await?;

        // Trim to u32 and check for overflow.
        let result = u32::try_from(result)?;
        // Return as f64, which converts to number type in Javascript.
        let result = f64::from(result);
        Ok(ConvexValue::try_from(result)?.into())
    }

    #[convex_macro::instrument_future]
    async fn get_user_identity(provider: &mut P, _args: JsonValue) -> anyhow::Result<JsonValue> {
        // TODO: Somehow make the Transaction aware of the dependency on the user.
        let tx = provider.tx()?;
        let user_identity = tx.user_identity();
        if let Some(user_identity) = user_identity {
            return user_identity.try_into();
        }

        Ok(JsonValue::Null)
    }

    #[convex_macro::instrument_future]
    async fn storage_generate_upload_url(
        provider: &mut P,
        _args: JsonValue,
    ) -> anyhow::Result<JsonValue> {
        let post_url = provider.file_storage_generate_upload_url()?;
        Ok(serde_json::to_value(post_url)?)
    }

    #[convex_macro::instrument_future]
    async fn storage_get_url(provider: &mut P, args: JsonValue) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct GetUrlArgs {
            storage_id: String,
        }
        let storage_id: FileStorageId = with_argument_error("storage.getUrl", || {
            let GetUrlArgs { storage_id } = serde_json::from_value(args)?;
            storage_id.parse().context(ArgName("storageId"))
        })?;
        let url = provider.file_storage_get_url(storage_id).await?;
        Ok(url.into())
    }

    #[convex_macro::instrument_future]
    async fn storage_delete(provider: &mut P, args: JsonValue) -> anyhow::Result<JsonValue> {
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
        provider.file_storage_delete(storage_id).await?;

        Ok(JsonValue::Null)
    }

    #[convex_macro::instrument_future]
    async fn storage_get_metadata(provider: &mut P, args: JsonValue) -> anyhow::Result<JsonValue> {
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
        let file_metadata = provider.file_storage_get_entry(storage_id).await?.map(
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
    async fn schedule(provider: &mut P, args: JsonValue) -> anyhow::Result<JsonValue> {
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

        let scheduled_ts = UnixTimestamp::from_secs_f64(ts);
        let (udf_path, udf_args) = provider
            .validate_schedule_args(udf_path, args.into_arg_vec(), scheduled_ts)
            .await?;

        let context = provider.context().clone();
        let tx = provider.tx()?;
        let virtual_id = VirtualSchedulerModel::new(tx)
            .schedule(udf_path, udf_args, scheduled_ts, context)
            .await?;

        Ok(JsonValue::from(virtual_id))
    }

    #[convex_macro::instrument_future]
    async fn cancel_job(provider: &mut P, args: JsonValue) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct CancelJobArgs {
            id: String,
        }
        let tx = provider.tx()?;

        let virtual_id_v6 = with_argument_error("db.cancel_job", || {
            let args: CancelJobArgs = serde_json::from_value(args)?;
            let id = DocumentIdV6::decode(&args.id).context(ArgName("id"))?;
            Ok(id)
        })?;

        VirtualSchedulerModel::new(tx).cancel(virtual_id_v6).await?;

        Ok(JsonValue::Null)
    }

    #[minitrace::trace]
    #[convex_macro::instrument_future]
    async fn insert(provider: &mut P, args: JsonValue) -> anyhow::Result<JsonValue> {
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
        let tx = provider.tx()?;
        let document_id = UserFacingModel::new(tx).insert(table, value).await?;
        let id_str = document_id.encode();
        Ok(json!({ "_id": id_str }))
    }

    #[minitrace::trace]
    #[convex_macro::instrument_future]
    async fn shallow_merge(provider: &mut P, args: JsonValue) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct UpdateArgs {
            id: String,
            value: JsonValue,
        }
        let table_filter = provider.table_filter();
        let tx = provider.tx()?;
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

    #[minitrace::trace]
    #[convex_macro::instrument_future]
    async fn replace(provider: &mut P, args: JsonValue) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ReplaceArgs {
            id: String,
            value: JsonValue,
        }
        let table_filter = provider.table_filter();
        let tx = provider.tx()?;
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

    #[minitrace::trace]
    #[convex_macro::instrument_future]
    async fn query_batch(
        provider: &mut P,
        batch_args: Vec<AsyncRead>,
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
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct QueryStreamNextArgs {
            // not dead code, clippy is wrong.
            #[allow(dead_code)]
            query_id: u32,
        }

        let table_filter = provider.table_filter();
        let mut queries_to_fetch = BTreeMap::new();
        let mut results = BTreeMap::new();
        let batch_size = batch_args.len();
        for (idx, args) in batch_args.into_iter().enumerate() {
            let result: anyhow::Result<_> = try {
                match args {
                    AsyncRead::QueryStreamNext(args) => {
                        let query_id = with_argument_error("queryStreamNext", || {
                            let args: QueryStreamNextArgs = serde_json::from_value(args)?;
                            Ok(args.query_id)
                        })?;
                        let local_query = provider
                            .query_manager()
                            .take_developer(query_id)
                            .context(ErrorMetadata::not_found(
                                "QueryNotFound",
                                "in-progress query not found",
                            ))?;
                        Some((Some(query_id), local_query))
                    },
                    AsyncRead::Get(args) => {
                        let tx = provider.tx()?;
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
                            Ok(table_name) => {
                                let query = Query::index_range(IndexRange {
                                    index_name: IndexName::by_id(table_name),
                                    range: vec![IndexRangeExpression::Eq(
                                        ID_FIELD_PATH.clone(),
                                        maybe_val!(id.encode()),
                                    )],
                                    order: Order::Asc,
                                });
                                Some((
                                    None,
                                    DeveloperQuery::new_with_version(
                                        tx,
                                        query,
                                        version,
                                        table_filter,
                                    )?,
                                ))
                            },
                            Err(_) => {
                                // Get on a non-existent table should return
                                // null.
                                None
                            },
                        }
                    },
                }
            };
            match result {
                Err(e) => {
                    assert!(results.insert(idx, Err(e)).is_none());
                },
                Ok(Some((query_id, query_to_fetch))) => {
                    assert!(queries_to_fetch
                        .insert(idx, (query_id, query_to_fetch))
                        .is_none());
                },
                Ok(None) => {
                    assert!(results.insert(idx, Ok(JsonValue::Null)).is_none());
                },
            }
        }

        let tx = match provider.tx() {
            Ok(tx) => tx,
            Err(e) => {
                return (0..batch_size).map(|_| Err(e.clone().into())).collect_vec();
            },
        };

        let mut fetch_results = query_batch_next(
            queries_to_fetch
                .iter_mut()
                .map(|(idx, (_, local_query))| (*idx, (local_query, None)))
                .collect(),
            tx,
        )
        .await;

        #[derive(Serialize)]
        struct QueryStreamNextResult {
            value: JsonValue,
            done: bool,
        }

        for (batch_key, (query_id, local_query)) in queries_to_fetch {
            let result: anyhow::Result<_> = try {
                if let Some(query_id) = query_id {
                    provider
                        .query_manager()
                        .insert_developer(query_id, local_query);
                }
                let maybe_next = fetch_results
                    .remove(&batch_key)
                    .context("batch_key missing")??;

                let done = maybe_next.is_none();
                let value = match maybe_next {
                    Some((doc, _)) => doc.into_value().0.into(),
                    None => ConvexValue::Null,
                };

                if let Some(query_id) = query_id {
                    if done {
                        provider.query_manager().cleanup_developer(query_id);
                    }
                    serde_json::to_value(QueryStreamNextResult {
                        value: value.into(),
                        done,
                    })?
                } else {
                    value.into()
                }
            };
            results.insert(batch_key, result);
        }
        assert_eq!(results.len(), batch_size);
        results.into_values().collect()
    }

    #[minitrace::trace]
    #[convex_macro::instrument_future]
    async fn query_page(provider: &mut P, args: JsonValue) -> anyhow::Result<JsonValue> {
        DatabaseSyscallsShared::query_page(provider, args).await
    }

    #[minitrace::trace]
    #[convex_macro::instrument_future]
    async fn remove(provider: &mut P, args: JsonValue) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RemoveArgs {
            id: String,
        }

        let table_filter = provider.table_filter();
        let tx = provider.tx()?;
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

struct DatabaseSyscallsShared<RT: Runtime, P: AsyncSyscallProvider<RT>> {
    _pd: PhantomData<(RT, P)>,
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

impl<RT: Runtime, P: AsyncSyscallProvider<RT>> DatabaseSyscallsShared<RT, P> {
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

    #[minitrace::trace]
    async fn query_page(provider: &mut P, args: JsonValue) -> anyhow::Result<JsonValue> {
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
        let table_filter = provider.table_filter();

        let page_size = args.page_size;
        if page_size == 0 {
            anyhow::bail!(ErrorMetadata::bad_request(
                "NoDocumentsForPagination",
                "Must request at least 1 document while paginating"
            ));
        }

        let start_cursor = args
            .cursor
            .map(|c| {
                provider
                    .key_broker()
                    .decrypt_cursor(c, provider.persistence_version())
            })
            .transpose()?;

        let end_cursor = match args.end_cursor {
            Some(end_cursor) => Some(
                provider
                    .key_broker()
                    .decrypt_cursor(end_cursor, provider.persistence_version())?,
            ),
            None => provider.prev_journal().end_cursor.clone(),
        };

        let tx = provider.tx()?;

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
            provider
                .key_broker()
                .encrypt_cursor(&split, provider.persistence_version())
        });

        let continue_cursor = match &cursor {
            None => anyhow::bail!(
                "Cursor was None. This should be impossible if `.next` was called on the query."
            ),
            Some(cursor) => provider
                .key_broker()
                .encrypt_cursor(cursor, provider.persistence_version()),
        };

        let is_done = matches!(
            cursor,
            Some(Cursor {
                position: CursorPosition::End,
                ..
            })
        );

        anyhow::ensure!(
            provider.next_journal().end_cursor.is_none(),
            ErrorMetadata::bad_request(
                "MultiplePaginatedDatabaseQueries",
                "This query or mutation function ran multiple paginated queries. Convex only \
                 supports a single paginated query in each function.",
            )
        );
        provider.next_journal().end_cursor = cursor;

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
