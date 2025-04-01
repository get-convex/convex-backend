#![allow(non_snake_case)]
use std::{
    collections::BTreeMap,
    marker::PhantomData,
    time::Duration,
};

use anyhow::Context;
use common::{
    bootstrap_model::components::handles::FunctionHandle,
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentId,
        PublicFunctionPath,
        Reference,
        ResolvedComponentFunctionPath,
        Resource,
    },
    document::DeveloperDocument,
    execution_context::ExecutionContext,
    knobs::{
        MAX_REACTOR_CALL_DEPTH,
        MAX_SYSCALL_BATCH_SIZE,
    },
    query::{
        Cursor,
        CursorPosition,
        Query,
    },
    query_journal::QueryJournal,
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    types::{
        AllowedVisibility,
        PersistenceVersion,
        UdfType,
    },
    value::ConvexValue,
    version::Version,
};
use database::{
    query::{
        query_batch_next,
        PaginationOptions,
        TableFilter,
    },
    soft_data_limit,
    table_summary::table_summary_bootstrapping_error,
    BootstrapComponentsModel,
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
    components::{
        handles::FunctionHandlesModel,
        ComponentsModel,
    },
    file_storage::{
        types::FileStorageEntry,
        BatchKey,
        FileStorageId,
    },
    scheduled_jobs::VirtualSchedulerModel,
    virtual_system_mapping,
};
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::{
    json,
    Value as JsonValue,
};
use udf::{
    validation::{
        validate_schedule_args,
        ValidatedPathAndArgs,
    },
    FunctionOutcome,
    UdfOutcome,
};
use value::{
    heap_size::HeapSize,
    id_v6::DeveloperDocumentId,
    ConvexArray,
    ConvexObject,
    TableName,
};

use super::DatabaseUdfEnvironment;
use crate::{
    client::EnvironmentData,
    environment::{
        action::parse_name_or_reference,
        helpers::{
            parse_version,
            with_argument_error,
            ArgName,
        },
    },
    helpers::UdfArgsJson,
    isolate2::client::QueryId,
    metrics::{
        async_syscall_timer,
        log_component_get_user_identity,
        log_run_udf,
    },
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
    StorageGetUrls(Vec<JsonValue>),
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
            "1.0/storageGetUrl" => Self::StorageGetUrls(vec![args]),
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
            (Self::StorageGetUrls(_), "1.0/storageGetUrl") => true,
            (Self::StorageGetUrls(_), _) => false,
            (Self::Unbatched { .. }, _) => false,
        }
    }

    pub fn push(&mut self, name: String, args: JsonValue) -> anyhow::Result<()> {
        match (&mut *self, &*name) {
            (Self::Reads(batch_args), "1.0/get") => batch_args.push(AsyncRead::Get(args)),
            (Self::Reads(batch_args), "1.0/queryStreamNext") => {
                batch_args.push(AsyncRead::QueryStreamNext(args))
            },
            (Self::StorageGetUrls(batch_args), "1.0/storageGetUrl") => {
                batch_args.push(args);
            },
            _ => anyhow::bail!("cannot push {name} onto {self:?}"),
        }
        Ok(())
    }

    pub fn name(&self) -> &str {
        match self {
            // 1.0/get is grouped in with 1.0/queryStreamNext.
            Self::Reads(_) => "1.0/queryStreamNext",
            Self::StorageGetUrls(_) => "1.0/storageGetUrl",
            Self::Unbatched { name, .. } => name,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Reads(args) => args.len(),
            Self::StorageGetUrls(args) => args.len(),
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

pub enum ManagedQuery<RT: Runtime> {
    Pending {
        query: Query,
        version: Option<Version>,
    },
    Active(DeveloperQuery<RT>),
}

// Trait for allowing code reuse between `DatabaseUdfEnvironment` and isolate2.
#[allow(async_fn_in_trait)]
pub trait AsyncSyscallProvider<RT: Runtime> {
    fn rt(&self) -> &RT;
    fn tx(&mut self) -> anyhow::Result<&mut Transaction<RT>>;
    fn key_broker(&self) -> &KeyBroker;
    fn context(&self) -> &ExecutionContext;

    fn observe_identity(&mut self) -> anyhow::Result<()>;

    fn persistence_version(&self) -> PersistenceVersion;
    fn is_system(&self) -> bool;
    fn table_filter(&self) -> TableFilter;
    fn component(&self) -> anyhow::Result<ComponentId>;

    fn log_async_syscall(&mut self, name: String, duration: Duration, is_success: bool);

    fn take_query(&mut self, query_id: QueryId) -> Option<ManagedQuery<RT>>;
    fn insert_query(&mut self, query_id: QueryId, query: DeveloperQuery<RT>);
    fn cleanup_query(&mut self, query_id: QueryId) -> bool;

    fn prev_journal(&mut self) -> &mut QueryJournal;
    fn next_journal(&mut self) -> &mut QueryJournal;

    async fn validate_schedule_args(
        &mut self,
        path: CanonicalizedComponentFunctionPath,
        args: Vec<JsonValue>,
        scheduled_ts: UnixTimestamp,
    ) -> anyhow::Result<(CanonicalizedComponentFunctionPath, ConvexArray)>;

    async fn file_storage_generate_upload_url(&mut self) -> anyhow::Result<String>;
    async fn file_storage_get_url_batch(
        &mut self,
        storage_ids: BTreeMap<BatchKey, FileStorageId>,
    ) -> BTreeMap<BatchKey, anyhow::Result<Option<String>>>;
    async fn file_storage_delete(&mut self, storage_id: FileStorageId) -> anyhow::Result<()>;
    async fn file_storage_get_entry(
        &mut self,
        storage_id: FileStorageId,
    ) -> anyhow::Result<Option<FileStorageEntry>>;

    async fn run_udf(
        &mut self,
        udf_type: UdfType,
        path: ResolvedComponentFunctionPath,
        args: ConvexObject,
    ) -> anyhow::Result<ConvexValue>;

    async fn create_function_handle(
        &mut self,
        path: CanonicalizedComponentFunctionPath,
    ) -> anyhow::Result<FunctionHandle>;

    async fn resolve(&mut self, reference: Reference) -> anyhow::Result<Resource>;
    async fn lookup_function_handle(
        &mut self,
        handle: FunctionHandle,
    ) -> anyhow::Result<CanonicalizedComponentFunctionPath>;
}

impl<RT: Runtime> AsyncSyscallProvider<RT> for DatabaseUdfEnvironment<RT> {
    fn rt(&self) -> &RT {
        &self.phase.rt
    }

    fn tx(&mut self) -> anyhow::Result<&mut Transaction<RT>> {
        self.phase.tx()
    }

    fn component(&self) -> anyhow::Result<ComponentId> {
        self.phase.component()
    }

    fn key_broker(&self) -> &KeyBroker {
        &self.key_broker
    }

    fn context(&self) -> &ExecutionContext {
        &self.context
    }

    fn observe_identity(&mut self) -> anyhow::Result<()> {
        self.phase.observe_identity()
    }

    fn persistence_version(&self) -> PersistenceVersion {
        self.persistence_version
    }

    fn is_system(&self) -> bool {
        self.path.udf_path.is_system()
    }

    fn table_filter(&self) -> TableFilter {
        if self.path.udf_path.is_system() {
            TableFilter::IncludePrivateSystemTables
        } else {
            TableFilter::ExcludePrivateSystemTables
        }
    }

    fn log_async_syscall(&mut self, name: String, duration: Duration, is_success: bool) {
        self.syscall_trace
            .log_async_syscall(name, duration, is_success);
    }

    fn take_query(&mut self, query_id: QueryId) -> Option<ManagedQuery<RT>> {
        self.query_manager
            .take_developer(query_id)
            .map(ManagedQuery::Active)
    }

    fn insert_query(&mut self, query_id: QueryId, query: DeveloperQuery<RT>) {
        self.query_manager.insert_developer(query_id, query);
    }

    fn cleanup_query(&mut self, query_id: QueryId) -> bool {
        self.query_manager.cleanup_developer(query_id)
    }

    fn prev_journal(&mut self) -> &mut QueryJournal {
        &mut self.prev_journal
    }

    fn next_journal(&mut self) -> &mut QueryJournal {
        &mut self.next_journal
    }

    async fn validate_schedule_args(
        &mut self,
        path: CanonicalizedComponentFunctionPath,
        args: Vec<JsonValue>,
        scheduled_ts: UnixTimestamp,
    ) -> anyhow::Result<(CanonicalizedComponentFunctionPath, ConvexArray)> {
        validate_schedule_args(
            path,
            args,
            scheduled_ts,
            self.phase.unix_timestamp()?,
            self.phase.tx()?,
        )
        .await
    }

    async fn file_storage_generate_upload_url(&mut self) -> anyhow::Result<String> {
        let issued_ts = self.phase.unix_timestamp()?;
        let component = self.component()?;
        let post_url = self
            .file_storage
            .generate_upload_url(self.phase.tx()?, &self.key_broker, issued_ts, component)
            .await?;
        Ok(post_url)
    }

    async fn file_storage_get_url_batch(
        &mut self,
        storage_ids: BTreeMap<BatchKey, FileStorageId>,
    ) -> BTreeMap<BatchKey, anyhow::Result<Option<String>>> {
        let component = match self.component() {
            Ok(c) => c,
            Err(e) => {
                return storage_ids
                    .into_keys()
                    .map(|batch_key| (batch_key, Err(e.clone_error())))
                    .collect();
            },
        };
        let tx = match self.phase.tx() {
            Ok(tx) => tx,
            Err(e) => {
                return storage_ids
                    .into_keys()
                    .map(|batch_key| (batch_key, Err(e.clone_error())))
                    .collect();
            },
        };
        self.file_storage
            .get_url_batch(tx, component, storage_ids)
            .await
    }

    async fn file_storage_delete(&mut self, storage_id: FileStorageId) -> anyhow::Result<()> {
        let component = self.component()?;
        self.file_storage
            .delete(self.phase.tx()?, component.into(), storage_id)
            .await
    }

    async fn file_storage_get_entry(
        &mut self,
        storage_id: FileStorageId,
    ) -> anyhow::Result<Option<FileStorageEntry>> {
        let component = self.component()?;
        self.file_storage
            .get_file_entry(self.phase.tx()?, component.into(), storage_id)
            .await
    }

    #[fastrace::trace]
    async fn run_udf(
        &mut self,
        udf_type: UdfType,
        path: ResolvedComponentFunctionPath,
        args: ConvexObject,
    ) -> anyhow::Result<ConvexValue> {
        match (self.udf_type, udf_type) {
            // Queries can call other queries.
            (UdfType::Query, UdfType::Query) => (),
            // Mutations can call queries or mutations,
            (UdfType::Mutation, UdfType::Query | UdfType::Mutation) => (),
            _ => {
                anyhow::bail!(ErrorMetadata::bad_request(
                    "InvalidFunctionCall",
                    format!(
                        "Cannot call a {} function from a {} function",
                        udf_type, self.udf_type
                    )
                ));
            },
        }
        let tx = self.phase.tx()?;
        let called_component_id = path.component;

        let path_and_args_result = ValidatedPathAndArgs::new_with_returns_validator(
            AllowedVisibility::All,
            tx,
            PublicFunctionPath::ResolvedComponent(path.clone()),
            ConvexArray::try_from(vec![args.into()])?,
            udf_type,
        )
        .await?;
        let (path_and_args, returns_validator) = match path_and_args_result {
            Ok(r) => r,
            Err(e) => {
                // TODO: Propagate this JsError to user space correctly.
                anyhow::bail!(ErrorMetadata::bad_request("InvalidArgs", e.message));
            },
        };

        // NB: Since this is a user error, we need to do this check before we take the
        // transaction below.
        let new_reactor_depth = if matches!(udf_type, UdfType::Query | UdfType::Mutation) {
            if self.reactor_depth >= *MAX_REACTOR_CALL_DEPTH {
                anyhow::bail!(ErrorMetadata::bad_request(
                    "MaximumCallDepthExceeded",
                    "Cross component call depth limit exceeded. Do you have an infinite loop in \
                     your app?"
                ));
            }
            self.reactor_depth + 1
        } else {
            0
        };

        let mut tx = self.phase.take_tx()?;
        let tokens = tx.begin_subtransaction();

        let query_journal = if self.is_system() && udf_type == UdfType::Query {
            self.prev_journal.clone()
        } else {
            QueryJournal::new()
        };
        let (mut tx, outcome) = self
            .udf_callback
            .execute_udf(
                self.client_id.clone(),
                udf_type,
                path_and_args,
                EnvironmentData {
                    key_broker: self.key_broker.clone(),
                    default_system_env_vars: BTreeMap::new(),
                    file_storage: self.file_storage.clone(),
                    module_loader: self.phase.module_loader().clone(),
                },
                tx,
                query_journal,
                self.context.clone(),
                new_reactor_depth,
            )
            .await?;
        match (udf_type, &outcome) {
            (UdfType::Mutation, FunctionOutcome::Mutation(UdfOutcome { result: Err(_), .. })) => {
                tx.rollback_subtransaction(tokens)?
            },
            _ => tx.commit_subtransaction(tokens)?,
        }
        self.phase.put_tx(tx)?;

        let outcome = match (udf_type, outcome) {
            (UdfType::Query, FunctionOutcome::Query(outcome))
            | (UdfType::Mutation, FunctionOutcome::Mutation(outcome)) => outcome,
            _ => anyhow::bail!("Unexpected outcome for {udf_type:?}"),
        };

        let UdfOutcome {
            result,
            observed_identity,
            // TODO: initialize the inner UDF's seed from the outer RNG seed
            observed_rng: _,
            // TODO: use the same timestamp for the inner UDF as the outer
            observed_time: _,
            // TODO: consider propagating syscall traces
            syscall_trace: _,
            log_lines,
            journal,
            arguments: _,
            identity: _,
            path: _,
            udf_server_version: _,
            unix_timestamp: _,
            rng_seed: _,
        } = outcome;

        log_run_udf(
            self.udf_type,
            udf_type,
            self.phase.observed_identity(),
            observed_identity,
        );

        if observed_identity {
            self.observe_identity()?;
        }

        if self.is_system() && udf_type == UdfType::Query && result.is_ok() {
            self.next_journal = journal;
        }

        // TODO(ENG-7663): restrict log lines within subfunctions instead of
        // limiting them only when they are returned to the parent.
        self.emit_sub_function_log_lines(path.for_logging(), log_lines);

        let result = match result {
            Ok(r) => r.unpack(),
            Err(e) => {
                // TODO: How do we want to propagate stack traces between component calls?
                anyhow::bail!(e);
            },
        };
        let tx = self.phase.tx()?;
        let table_mapping = tx.table_mapping().namespace(called_component_id.into());
        if let Some(e) =
            returns_validator.check_output(&result, &table_mapping, virtual_system_mapping())
        {
            anyhow::bail!(ErrorMetadata::bad_request("InvalidReturnValue", e.message));
        }
        Ok(result)
    }

    async fn create_function_handle(
        &mut self,
        path: CanonicalizedComponentFunctionPath,
    ) -> anyhow::Result<FunctionHandle> {
        let tx = self.phase.tx()?;
        FunctionHandlesModel::new(tx)
            .get_with_component_path(path)
            .await
    }

    async fn resolve(&mut self, reference: Reference) -> anyhow::Result<Resource> {
        let current_component_id = self.component()?;
        let current_udf_path = self.path.udf_path.clone().into();

        let tx = self.phase.tx()?;

        ComponentsModel::new(tx)
            .resolve(current_component_id, Some(current_udf_path), &reference)
            .await
    }

    async fn lookup_function_handle(
        &mut self,
        handle: FunctionHandle,
    ) -> anyhow::Result<CanonicalizedComponentFunctionPath> {
        FunctionHandlesModel::new(self.phase.tx()?)
            .lookup(handle)
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
    #[fastrace::trace]
    pub async fn run_async_syscall_batch(
        provider: &mut P,
        batch: AsyncSyscallBatch,
    ) -> Vec<anyhow::Result<String>> {
        let start = provider.rt().monotonic_now();
        let batch_name = batch.name().to_string();
        let timer = async_syscall_timer(&batch_name);
        // Outer error is a system error that encompases the whole batch, while
        // inner errors are for individual batch items that may be system or developer
        // errors.
        let results = match batch {
            AsyncSyscallBatch::Reads(batch_args) => Self::query_batch(provider, batch_args).await,
            AsyncSyscallBatch::StorageGetUrls(batch_args) => {
                Self::storage_get_url_batch(provider, batch_args).await
            },
            AsyncSyscallBatch::Unbatched { name, args } => {
                let result = match &name[..] {
                    // Database
                    "1.0/count" => Box::pin(Self::count(provider, args)).await,
                    "1.0/insert" => Box::pin(Self::insert(provider, args)).await,
                    "1.0/shallowMerge" => Box::pin(Self::shallow_merge(provider, args)).await,
                    "1.0/replace" => Box::pin(Self::replace(provider, args)).await,
                    "1.0/remove" => Box::pin(Self::remove(provider, args)).await,
                    "1.0/queryPage" => Box::pin(Self::query_page(provider, args)).await,
                    // Auth
                    "1.0/getUserIdentity" => {
                        Box::pin(Self::get_user_identity(provider, args)).await
                    },
                    // Storage
                    "1.0/storageDelete" => Box::pin(Self::storage_delete(provider, args)).await,
                    "1.0/storageGetMetadata" => {
                        Box::pin(Self::storage_get_metadata(provider, args)).await
                    },
                    "1.0/storageGenerateUploadUrl" => {
                        Box::pin(Self::storage_generate_upload_url(provider, args)).await
                    },
                    // Scheduling
                    "1.0/schedule" => Box::pin(Self::schedule(provider, args)).await,
                    "1.0/cancel_job" => Box::pin(Self::cancel_job(provider, args)).await,

                    // Components
                    "1.0/runUdf" => Box::pin(Self::run_udf(provider, args)).await,
                    "1.0/createFunctionHandle" => {
                        Box::pin(Self::create_function_handle(provider, args)).await
                    },

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
        results
            .into_iter()
            .map(|result| anyhow::Ok(serde_json::to_string(&result?)?))
            .collect()
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
        let component = provider.component()?;
        let tx = provider.tx()?;
        let result = tx.count(component.into(), &table).await?;
        let Some(result) = result else {
            return Err(table_summary_bootstrapping_error(Some(
                "Table count unavailable while bootstrapping",
            )));
        };

        // Trim to u32 and check for overflow.
        let result = u32::try_from(result)?;
        // Return as f64, which converts to number type in Javascript.
        let result = f64::from(result);
        Ok(ConvexValue::from(result).to_internal_json())
    }

    #[convex_macro::instrument_future]
    async fn get_user_identity(provider: &mut P, _args: JsonValue) -> anyhow::Result<JsonValue> {
        provider.observe_identity()?;
        // TODO: Somehow make the Transaction aware of the dependency on the user.
        let tx = provider.tx()?;
        let user_identity = tx.user_identity();
        if !provider.component()?.is_root() {
            log_component_get_user_identity(user_identity.is_some());
        }
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
        let post_url = provider.file_storage_generate_upload_url().await?;
        Ok(serde_json::to_value(post_url)?)
    }

    #[convex_macro::instrument_future]
    async fn storage_get_url_batch(
        provider: &mut P,
        batch_args: Vec<JsonValue>,
    ) -> Vec<anyhow::Result<JsonValue>> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct GetUrlArgs {
            storage_id: String,
        }
        let batch_size = batch_args.len();
        let mut results = BTreeMap::new();
        let mut storage_ids = BTreeMap::new();
        for (idx, args) in batch_args.into_iter().enumerate() {
            let storage_id_result = with_argument_error("storage.getUrl", || {
                let GetUrlArgs { storage_id } = serde_json::from_value(args)?;
                storage_id.parse().context(ArgName("storageId"))
            });
            match storage_id_result {
                Ok(storage_id) => {
                    storage_ids.insert(idx, storage_id);
                },
                Err(e) => {
                    assert!(results.insert(idx, Err(e)).is_none());
                },
            }
        }
        let urls = provider.file_storage_get_url_batch(storage_ids).await;
        for (batch_key, url) in urls {
            assert!(results
                .insert(batch_key, url.map(JsonValue::from))
                .is_none());
        }
        assert_eq!(results.len(), batch_size);
        results.into_values().collect()
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
                provider.lookup_function_handle(handle).await?
            },
            None => {
                let reference = parse_name_or_reference("scheduler", name, reference)?;
                match provider.resolve(reference).await? {
                    Resource::Value(v) => {
                        anyhow::bail!(ErrorMetadata::bad_request(
                            "InvalidResource",
                            format!(
                                "Only functions can be scheduled. {} is not a function",
                                v.to_internal_json()
                            ),
                        ));
                    },
                    Resource::Function(p) => p,
                    Resource::ResolvedSystemUdf { .. } => {
                        anyhow::bail!("Cannot schedule function by component id");
                    },
                }
            },
        };

        let scheduling_component = provider.component()?;

        let scheduled_ts = UnixTimestamp::from_secs_f64(ts);
        let (path, udf_args) = provider
            .validate_schedule_args(path, args.into_arg_vec(), scheduled_ts)
            .await?;

        let context = provider.context().clone();
        let tx = provider.tx()?;
        let virtual_id = VirtualSchedulerModel::new(tx, scheduling_component.into())
            .schedule(path, udf_args, scheduled_ts, context)
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
        let component = provider.component()?;
        let tx = provider.tx()?;

        let virtual_id_v6 = with_argument_error("db.cancel_job", || {
            let args: CancelJobArgs = serde_json::from_value(args)?;
            let id = DeveloperDocumentId::decode(&args.id).context(ArgName("id"))?;
            Ok(id)
        })?;

        VirtualSchedulerModel::new(tx, component.into())
            .cancel(virtual_id_v6)
            .await?;

        Ok(JsonValue::Null)
    }

    #[fastrace::trace]
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
        let component = provider.component()?;
        let tx = provider.tx()?;
        let document_id = UserFacingModel::new(tx, component.into())
            .insert(table, value)
            .await?;
        let id_str = document_id.encode();
        Ok(json!({ "_id": id_str }))
    }

    #[fastrace::trace]
    #[convex_macro::instrument_future]
    async fn shallow_merge(provider: &mut P, args: JsonValue) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct UpdateArgs {
            id: String,
            value: JsonValue,
        }
        let table_filter = provider.table_filter();
        let component = provider.component()?;
        let tx = provider.tx()?;
        let (id, value, table_name) = with_argument_error("db.patch", || {
            let args: UpdateArgs = serde_json::from_value(args)?;

            let id = DeveloperDocumentId::decode(&args.id).context(ArgName("id"))?;
            let table_name = tx
                .resolve_idv6(id, component.into(), table_filter)
                .context(ArgName("id"))?;

            let value = PatchValue::try_from(args.value).context(ArgName("value"))?;
            Ok((id, value, table_name))
        })?;

        system_table_guard(&table_name, false)?;

        let document = UserFacingModel::new(tx, component.into())
            .patch(id, value)
            .await?;
        Ok(document.to_internal_json())
    }

    #[fastrace::trace]
    #[convex_macro::instrument_future]
    async fn replace(provider: &mut P, args: JsonValue) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ReplaceArgs {
            id: String,
            value: JsonValue,
        }
        let table_filter = provider.table_filter();
        let component = provider.component()?;
        let tx = provider.tx()?;
        let (id, value, table_name) = with_argument_error("db.replace", || {
            let args: ReplaceArgs = serde_json::from_value(args)?;

            let id = DeveloperDocumentId::decode(&args.id).context(ArgName("id"))?;
            let table_name = tx
                .resolve_idv6(id, component.into(), table_filter)
                .context(ArgName("id"))?;

            let value = ConvexValue::try_from(args.value).context(ArgName("value"))?;
            Ok((id, value.try_into().context(ArgName("value"))?, table_name))
        })?;

        system_table_guard(&table_name, false)?;

        let document = UserFacingModel::new(tx, component.into())
            .replace(id, value)
            .await?;
        Ok(document.to_internal_json())
    }

    #[fastrace::trace]
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
                        let managed_query =
                            provider
                                .take_query(query_id)
                                .context(ErrorMetadata::bad_request(
                                    "QueryNotFound",
                                    "in-progress query not found",
                                ))?;
                        let local_query = match managed_query {
                            ManagedQuery::Pending { query, version } => {
                                let component = provider.component()?;
                                DeveloperQuery::new_with_version(
                                    provider.tx()?,
                                    component.into(),
                                    query,
                                    version,
                                    table_filter,
                                )?
                            },
                            ManagedQuery::Active(local_query) => local_query,
                        };
                        Some((Some(query_id), local_query))
                    },
                    AsyncRead::Get(args) => {
                        let component = provider.component()?;
                        let tx = provider.tx()?;
                        let (id, is_system, version) = with_argument_error("db.get", || {
                            let args: GetArgs = serde_json::from_value(args)?;
                            let id =
                                DeveloperDocumentId::decode(&args.id).context(ArgName("id"))?;
                            let version = parse_version(args.version)?;
                            Ok((id, args.is_system, version))
                        })?;
                        let name: Result<TableName, anyhow::Error> = tx
                            .all_tables_number_to_name(component.into(), table_filter)(
                            id.table()
                        );
                        if name.is_ok() {
                            system_table_guard(&name?, is_system)?;
                        }
                        match tx.resolve_idv6(id, component.into(), table_filter) {
                            Ok(table_name) => {
                                let query = Query::get(table_name, id);
                                Some((
                                    None,
                                    DeveloperQuery::new_with_version(
                                        tx,
                                        component.into(),
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
                return (0..batch_size).map(|_| Err(e.clone_error())).collect_vec();
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
                    provider.insert_query(query_id, local_query);
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
                        provider.cleanup_query(query_id);
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

    #[fastrace::trace]
    #[convex_macro::instrument_future]
    async fn query_page(provider: &mut P, args: JsonValue) -> anyhow::Result<JsonValue> {
        DatabaseSyscallsShared::query_page(provider, args).await
    }

    #[fastrace::trace]
    #[convex_macro::instrument_future]
    async fn remove(provider: &mut P, args: JsonValue) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RemoveArgs {
            id: String,
        }

        let table_filter = provider.table_filter();
        let component = provider.component()?;
        let tx = provider.tx()?;
        let (id, table_name) = with_argument_error("db.delete", || {
            let args: RemoveArgs = serde_json::from_value(args)?;
            let id = DeveloperDocumentId::decode(&args.id).context(ArgName("id"))?;
            let table_name = tx
                .resolve_idv6(id, component.into(), table_filter)
                .context(ArgName("id"))?;
            Ok((id, table_name))
        })?;

        system_table_guard(&table_name, false)?;

        let document = UserFacingModel::new(tx, component.into())
            .delete(id)
            .await?;
        Ok(document.to_internal_json())
    }

    #[convex_macro::instrument_future]
    async fn run_udf(provider: &mut P, args: JsonValue) -> anyhow::Result<JsonValue> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RunUdfArgs {
            udf_type: String,
            name: Option<String>,
            reference: Option<String>,
            function_handle: Option<String>,
            args: JsonValue,
        }
        let RunUdfArgs {
            udf_type,
            name,
            reference,
            function_handle,
            args,
        } = with_argument_error("runUdf", || Ok(serde_json::from_value(args)?))?;
        let (udf_type, args) = with_argument_error("runUdf", || {
            let udf_type: UdfType = udf_type.parse().context(ArgName("udfType"))?;
            let args: ConvexObject = ConvexValue::try_from(args)
                .context(ArgName("args"))?
                .try_into()
                .context(ArgName("args"))?;
            Ok((udf_type, args))
        })?;
        let path = match function_handle {
            Some(function_handle) => {
                let handle: FunctionHandle =
                    with_argument_error("runUdf", || function_handle.parse())?;
                let path = provider.lookup_function_handle(handle).await?;
                let tx = provider.tx()?;
                let (_, component) = BootstrapComponentsModel::new(tx)
                    .must_component_path_to_ids(&path.component)?;
                ResolvedComponentFunctionPath {
                    component,
                    udf_path: path.udf_path,
                    component_path: Some(path.component),
                }
            },
            None => {
                let reference = parse_name_or_reference("runUdf", name, reference)?;
                let resource = provider.resolve(reference).await?;
                match resource {
                    Resource::ResolvedSystemUdf(path) => path,
                    Resource::Value(_) => {
                        anyhow::bail!(ErrorMetadata::bad_request(
                            "InvalidResource",
                            "Cannot execute a value resource"
                        ));
                    },
                    Resource::Function(path) => {
                        let tx = provider.tx()?;
                        let (_, component) = BootstrapComponentsModel::new(tx)
                            .must_component_path_to_ids(&path.component)?;
                        ResolvedComponentFunctionPath {
                            component,
                            udf_path: path.udf_path,
                            component_path: Some(path.component),
                        }
                    },
                }
            },
        };
        let value = provider.run_udf(udf_type, path, args).await?;
        Ok(value.into())
    }

    async fn create_function_handle(
        provider: &mut P,
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
                match provider.resolve(reference).await? {
                    Resource::Function(path) => path,
                    Resource::ResolvedSystemUdf { .. } => {
                        anyhow::bail!("Cannot create function handle for system UDF");
                    },
                    Resource::Value(_) => {
                        anyhow::bail!(ErrorMetadata::bad_request(
                            "InvalidResource",
                            "Cannot create a function handle for a value resource"
                        ));
                    },
                }
            },
        };
        let handle = provider.create_function_handle(function_path).await?;
        Ok(serde_json::to_value(String::from(handle))?)
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
    cursor: Cursor,
    split_cursor: Option<Cursor>,
    page_status: Option<QueryPageStatus>,
}

impl<RT: Runtime, P: AsyncSyscallProvider<RT>> DatabaseSyscallsShared<RT, P> {
    async fn read_page_from_query(
        mut query: DeveloperQuery<RT>,
        tx: &mut Transaction<RT>,
        page_size: usize,
    ) -> anyhow::Result<(Vec<DeveloperDocument>, QueryPageMetadata)> {
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
                        if query.cursor().is_none() {
                            // Intentionally drop ErrorMetadata because this should
                            // be impossible, so we want to throw a system error instead.
                            anyhow::bail!(
                                "This should be impossible. Hit pagination limit before setting \
                                 query cursor: {e:?}"
                            );
                        }
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
        let cursor = end_cursor.or_else(|| query.cursor()).context(
            "Cursor was None. This should be impossible if `.next` was called on the query.",
        )?;
        Ok((
            page,
            QueryPageMetadata {
                cursor,
                split_cursor: query.split_cursor(),
                page_status,
            },
        ))
    }

    #[fastrace::trace]
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
        if args.maximum_rows_read == Some(0) || args.maximum_bytes_read == Some(0) {
            anyhow::bail!(ErrorMetadata::bad_request(
                "InvalidPaginationLimit",
                "maximumRowsRead and maximumBytesRead must be greater than 0"
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

        let component = provider.component()?;
        if !component.is_root() && !provider.is_system() {
            anyhow::bail!(ErrorMetadata::bad_request(
                "PaginationUnsupportedInComponents",
                "paginate() is only supported in the app.",
            ));
        }

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
                component.into(),
                parsed_query,
                PaginationOptions::ReactivePagination {
                    start_cursor,
                    end_cursor,
                    maximum_rows_read: args.maximum_rows_read,
                    maximum_bytes_read: args.maximum_bytes_read,
                },
                version,
                table_filter,
            )?;
            let (page, metadata) = Self::read_page_from_query(query, tx, page_size).await?;
            let page = page.into_iter().map(|doc| doc.to_internal_json()).collect();
            (page, metadata)
        };

        let page_status = page_status.map(|s| s.as_str());

        // Place split_cursor in the middle.
        let split_cursor = split_cursor.map(|split| {
            provider
                .key_broker()
                .encrypt_cursor(&split, provider.persistence_version())
        });

        let continue_cursor = provider
            .key_broker()
            .encrypt_cursor(&cursor, provider.persistence_version());

        let is_done = matches!(
            cursor,
            Cursor {
                position: CursorPosition::End,
                ..
            }
        );

        anyhow::ensure!(
            provider.next_journal().end_cursor.is_none(),
            ErrorMetadata::bad_request(
                "MultiplePaginatedDatabaseQueries",
                "This query or mutation function ran multiple paginated queries. Convex only \
                 supports a single paginated query in each function.",
            )
        );
        provider.next_journal().end_cursor = Some(cursor);

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
