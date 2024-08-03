use std::{
    ops::Bound,
    time::Duration,
};

use async_trait::async_trait;
use bytes::Bytes;
use common::{
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentId,
    },
    http::ResolvedHost,
    pause::PauseClient,
    runtime::Runtime,
    types::{
        AllowedVisibility,
        ConvexOrigin,
        FunctionCaller,
        RepeatableTimestamp,
    },
    RequestId,
};
use database::{
    Database,
    LogReader,
    ReadSet,
    Subscription,
    Token,
};
use file_storage::{
    FileRangeStream,
    FileStream,
};
use futures::{
    future::BoxFuture,
    stream::BoxStream,
    FutureExt,
};
use headers::{
    ContentLength,
    ContentType,
};
use isolate::{
    HttpActionRequest,
    HttpActionResponseStreamer,
};
use keybroker::Identity;
use model::{
    file_storage::FileStorageId,
    session_requests::types::SessionRequestIdentifier,
};
use serde_json::Value as JsonValue;
use sync_types::{
    AuthenticationToken,
    SerializedQueryJournal,
    Timestamp,
};
use value::{
    sha256::Sha256Digest,
    DeveloperDocumentId,
};

use crate::{
    Application,
    FunctionError,
    FunctionReturn,
    RedactedActionError,
    RedactedActionReturn,
    RedactedMutationError,
    RedactedMutationReturn,
    RedactedQueryReturn,
};

#[cfg_attr(
    any(test, feature = "testing"),
    derive(proptest_derive::Arbitrary, Debug, Clone, PartialEq)
)]
pub enum ExecuteQueryTimestamp {
    // Execute the query at the latest timestamp.
    Latest,
    // Execute the query at a given timestamp.
    At(Timestamp),
}

// A trait that abstracts the backend API. It all state and validation logic
// so http routes can be kept thin and stateless. The implementor is also
// responsible for routing the request to the appropriate backend in the hosted
// version of Convex.
#[async_trait]
pub trait ApplicationApi: Send + Sync {
    async fn authenticate(
        &self,
        host: &ResolvedHost,
        request_id: RequestId,
        auth_token: AuthenticationToken,
    ) -> anyhow::Result<Identity>;

    async fn execute_public_query(
        &self,
        host: &ResolvedHost,
        request_id: RequestId,
        identity: Identity,
        path: CanonicalizedComponentFunctionPath,
        args: Vec<JsonValue>,
        caller: FunctionCaller,
        ts: ExecuteQueryTimestamp,
        journal: Option<SerializedQueryJournal>,
    ) -> anyhow::Result<RedactedQueryReturn>;

    async fn execute_public_mutation(
        &self,
        host: &ResolvedHost,
        request_id: RequestId,
        identity: Identity,
        path: CanonicalizedComponentFunctionPath,
        args: Vec<JsonValue>,
        caller: FunctionCaller,
        // Identifier used to make this mutation idempotent.
        mutation_identifier: Option<SessionRequestIdentifier>,
    ) -> anyhow::Result<Result<RedactedMutationReturn, RedactedMutationError>>;

    async fn execute_public_action(
        &self,
        host: &ResolvedHost,
        request_id: RequestId,
        identity: Identity,
        path: CanonicalizedComponentFunctionPath,
        args: Vec<JsonValue>,
        caller: FunctionCaller,
    ) -> anyhow::Result<Result<RedactedActionReturn, RedactedActionError>>;

    async fn execute_any_function(
        &self,
        host: &ResolvedHost,
        request_id: RequestId,
        identity: Identity,
        path: CanonicalizedComponentFunctionPath,
        args: Vec<JsonValue>,
        caller: FunctionCaller,
    ) -> anyhow::Result<Result<FunctionReturn, FunctionError>>;

    async fn latest_timestamp(
        &self,
        host: &ResolvedHost,
        request_id: RequestId,
    ) -> anyhow::Result<RepeatableTimestamp>;

    async fn execute_http_action(
        &self,
        host: &ResolvedHost,
        request_id: RequestId,
        http_request_metadata: HttpActionRequest,
        identity: Identity,
        caller: FunctionCaller,
        response_streamer: HttpActionResponseStreamer,
    ) -> anyhow::Result<()>;

    async fn check_store_file_authorization(
        &self,
        host: &ResolvedHost,
        request_id: RequestId,
        token: &str,
        validity: Duration,
    ) -> anyhow::Result<ComponentId>;

    async fn store_file(
        &self,
        host: &ResolvedHost,
        request_id: RequestId,
        origin: ConvexOrigin,
        component: ComponentId,
        content_length: Option<ContentLength>,
        content_type: Option<ContentType>,
        expected_sha256: Option<Sha256Digest>,
        body: BoxStream<'_, anyhow::Result<Bytes>>,
    ) -> anyhow::Result<DeveloperDocumentId>;

    async fn get_file_range(
        &self,
        host: &ResolvedHost,
        request_id: RequestId,
        origin: ConvexOrigin,
        component: ComponentId,
        file_storage_id: FileStorageId,
        range: (Bound<u64>, Bound<u64>),
    ) -> anyhow::Result<FileRangeStream>;

    async fn get_file(
        &self,
        host: &ResolvedHost,
        request_id: RequestId,
        origin: ConvexOrigin,
        component: ComponentId,
        file_storage_id: FileStorageId,
    ) -> anyhow::Result<FileStream>;

    // Returns a fallible subscription client. The implementation is not required to
    // recover from transient errors with the underlying connection or stream. The
    // client is responsible to Drop the client and create a new one on any system
    // errors. The intended use is to create a single client for every new web
    // socket connection. NOTE: We might eventually strengthen the requirement for
    // the implementation and require it to reconnect internally but easier to
    // start this way.
    async fn subscription_client(
        &self,
        host: &ResolvedHost,
    ) -> anyhow::Result<Box<dyn SubscriptionClient>>;
}

// Implements ApplicationApi via Application.
#[async_trait]
impl<RT: Runtime> ApplicationApi for Application<RT> {
    async fn authenticate(
        &self,
        _host: &ResolvedHost,
        _request_id: RequestId,
        auth_token: AuthenticationToken,
    ) -> anyhow::Result<Identity> {
        let validate_time = self.runtime().system_time();
        self.authenticate(auth_token, validate_time).await
    }

    async fn execute_public_query(
        &self,
        _host: &ResolvedHost,
        request_id: RequestId,
        identity: Identity,
        path: CanonicalizedComponentFunctionPath,
        args: Vec<JsonValue>,
        caller: FunctionCaller,
        ts: ExecuteQueryTimestamp,
        journal: Option<SerializedQueryJournal>,
    ) -> anyhow::Result<RedactedQueryReturn> {
        anyhow::ensure!(
            caller.allowed_visibility() == AllowedVisibility::PublicOnly,
            "This method should not be used by internal callers."
        );

        let ts = match ts {
            ExecuteQueryTimestamp::Latest => *self.now_ts_for_reads(),
            ExecuteQueryTimestamp::At(ts) => ts,
        };
        self.read_only_udf_at_ts(request_id, path, args, identity, ts, journal, caller)
            .await
    }

    async fn execute_public_mutation(
        &self,
        _host: &ResolvedHost,
        request_id: RequestId,
        identity: Identity,
        path: CanonicalizedComponentFunctionPath,
        args: Vec<JsonValue>,
        caller: FunctionCaller,
        // Identifier used to make this mutation idempotent.
        mutation_identifier: Option<SessionRequestIdentifier>,
    ) -> anyhow::Result<Result<RedactedMutationReturn, RedactedMutationError>> {
        anyhow::ensure!(
            caller.allowed_visibility() == AllowedVisibility::PublicOnly,
            "This method should not be used by internal callers."
        );

        self.mutation_udf(
            request_id,
            path,
            args,
            identity,
            mutation_identifier,
            caller,
            PauseClient::new(),
        )
        .await
    }

    async fn execute_public_action(
        &self,
        _host: &ResolvedHost,
        request_id: RequestId,
        identity: Identity,
        path: CanonicalizedComponentFunctionPath,
        args: Vec<JsonValue>,
        caller: FunctionCaller,
    ) -> anyhow::Result<Result<RedactedActionReturn, RedactedActionError>> {
        anyhow::ensure!(
            caller.allowed_visibility() == AllowedVisibility::PublicOnly,
            "This method should not be used by internal callers."
        );

        self.action_udf(request_id, path, args, identity, caller)
            .await
    }

    async fn execute_any_function(
        &self,
        _host: &ResolvedHost,
        request_id: RequestId,
        identity: Identity,
        path: CanonicalizedComponentFunctionPath,
        args: Vec<JsonValue>,
        caller: FunctionCaller,
    ) -> anyhow::Result<Result<FunctionReturn, FunctionError>> {
        self.any_udf(request_id, path, args, identity, caller).await
    }

    async fn latest_timestamp(
        &self,
        _host: &ResolvedHost,
        _request_id: RequestId,
    ) -> anyhow::Result<RepeatableTimestamp> {
        Ok(self.now_ts_for_reads())
    }

    async fn execute_http_action(
        &self,
        _host: &ResolvedHost,
        request_id: RequestId,
        http_request_metadata: HttpActionRequest,
        identity: Identity,
        caller: FunctionCaller,
        response_streamer: HttpActionResponseStreamer,
    ) -> anyhow::Result<()> {
        self.http_action_udf(
            request_id,
            http_request_metadata,
            identity,
            caller,
            response_streamer,
        )
        .await
    }

    async fn check_store_file_authorization(
        &self,
        _host: &ResolvedHost,
        _request_id: RequestId,
        token: &str,
        validity: Duration,
    ) -> anyhow::Result<ComponentId> {
        self.key_broker()
            .check_store_file_authorization(&self.runtime, token, validity)
    }

    async fn store_file(
        &self,
        _host: &ResolvedHost,
        _request_id: RequestId,
        _origin: ConvexOrigin,
        component: ComponentId,
        content_length: Option<ContentLength>,
        content_type: Option<ContentType>,
        expected_sha256: Option<Sha256Digest>,
        body: BoxStream<'_, anyhow::Result<Bytes>>,
    ) -> anyhow::Result<DeveloperDocumentId> {
        self.store_file(
            component,
            content_length,
            content_type,
            expected_sha256,
            body,
        )
        .await
    }

    async fn get_file_range(
        &self,
        _host: &ResolvedHost,
        _request_id: RequestId,
        _origin: ConvexOrigin,
        component: ComponentId,
        file_storage_id: FileStorageId,
        range: (Bound<u64>, Bound<u64>),
    ) -> anyhow::Result<FileRangeStream> {
        self.get_file_range(component, file_storage_id, range).await
    }

    async fn get_file(
        &self,
        _host: &ResolvedHost,
        _request_id: RequestId,
        _origin: ConvexOrigin,
        component: ComponentId,
        file_storage_id: FileStorageId,
    ) -> anyhow::Result<FileStream> {
        self.get_file(component, file_storage_id).await
    }

    async fn subscription_client(
        &self,
        _host: &ResolvedHost,
    ) -> anyhow::Result<Box<dyn SubscriptionClient>> {
        Ok(Box::new(ApplicationSubscriptionClient {
            database: self.database.clone(),
        }))
    }
}

#[async_trait]
pub trait SubscriptionClient: Send + Sync {
    async fn subscribe(&self, token: Token) -> anyhow::Result<Box<dyn SubscriptionTrait>>;
}

struct ApplicationSubscriptionClient<RT: Runtime> {
    database: Database<RT>,
}

#[async_trait]
impl<RT: Runtime> SubscriptionClient for ApplicationSubscriptionClient<RT> {
    async fn subscribe(&self, token: Token) -> anyhow::Result<Box<dyn SubscriptionTrait>> {
        let inner = self.database.subscribe(token.clone()).await?;
        Ok(Box::new(ApplicationSubscription {
            initial_ts: token.ts(),
            reads: token.into_reads(),
            inner,
            log: self.database.log().clone(),
        }))
    }
}

#[async_trait]
pub trait SubscriptionTrait: Send + Sync {
    fn wait_for_invalidation(&self) -> BoxFuture<'static, anyhow::Result<()>>;

    // Returns true if the subscription validity can be extended to new_ts. Note
    // that extend_validity might return false even if the subscription can be
    // extended, but will never return true if it can't.
    async fn extend_validity(&self, new_ts: Timestamp) -> anyhow::Result<bool>;
}

struct ApplicationSubscription {
    inner: Subscription,
    log: LogReader,

    reads: ReadSet,
    // The initial timestamp the subscription was created at. This is known
    // to be valid.
    initial_ts: Timestamp,
}

#[async_trait]
impl SubscriptionTrait for ApplicationSubscription {
    fn wait_for_invalidation(&self) -> BoxFuture<'static, anyhow::Result<()>> {
        self.inner.wait_for_invalidation().map(Ok).boxed()
    }

    async fn extend_validity(&self, new_ts: Timestamp) -> anyhow::Result<bool> {
        if new_ts < self.initial_ts {
            // new_ts is before the initial subscription timestamp.
            return Ok(false);
        }

        // The inner subscription is periodically updated by the subscription
        // worker.
        let Some(current_ts) = self.inner.current_ts() else {
            // Subscription is no longer valid. We could check validity from end_ts
            // to new_ts, but this is likely to fail and is potentially unbounded amount of
            // work, so we return false here. This is valid per the function contract.
            return Ok(false);
        };

        let current_token = Token::new(self.reads.clone(), current_ts);
        let Some(_new_token) = self.log.refresh_token(current_token, new_ts)? else {
            // Subscription validity can't be extended. Note that returning false
            // here also doesn't mean there is a conflict.
            return Ok(false);
        };
        return Ok(true);
    }
}
