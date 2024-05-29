use async_trait::async_trait;
use common::{
    components::{
        ComponentFunctionPath,
        ComponentPath,
    },
    pause::PauseClient,
    runtime::Runtime,
    types::{
        AllowedVisibility,
        FunctionCaller,
        RepeatableTimestamp,
    },
    RequestId,
};
use database::{
    LogReader,
    ReadSet,
    Subscription,
    Token,
};
use futures::{
    future::BoxFuture,
    FutureExt,
};
use model::session_requests::types::SessionRequestIdentifier;
use serde_json::Value as JsonValue;
use sync_types::{
    AuthenticationToken,
    SerializedQueryJournal,
    Timestamp,
    UdfPath,
};

use crate::{
    Application,
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
    async fn execute_public_query(
        &self,
        host: &str,
        request_id: RequestId,
        auth_token: AuthenticationToken,
        path: UdfPath,
        args: Vec<JsonValue>,
        caller: FunctionCaller,
        ts: ExecuteQueryTimestamp,
        journal: Option<SerializedQueryJournal>,
    ) -> anyhow::Result<RedactedQueryReturn>;

    async fn execute_public_mutation(
        &self,
        host: &str,
        request_id: RequestId,
        auth_token: AuthenticationToken,
        path: UdfPath,
        args: Vec<JsonValue>,
        caller: FunctionCaller,
        // Identifier used to make this mutation idempotent.
        mutation_identifier: Option<SessionRequestIdentifier>,
    ) -> anyhow::Result<Result<RedactedMutationReturn, RedactedMutationError>>;

    async fn execute_public_action(
        &self,
        host: &str,
        request_id: RequestId,
        auth_token: AuthenticationToken,
        path: UdfPath,
        args: Vec<JsonValue>,
        caller: FunctionCaller,
    ) -> anyhow::Result<Result<RedactedActionReturn, RedactedActionError>>;

    async fn latest_timestamp(
        &self,
        host: &str,
        request_id: RequestId,
    ) -> anyhow::Result<RepeatableTimestamp>;

    async fn subscribe(&self, token: Token) -> anyhow::Result<Box<dyn SubscriptionTrait>>;
}

// Implements ApplicationApi via Application.
#[async_trait]
impl<RT: Runtime> ApplicationApi for Application<RT> {
    async fn execute_public_query(
        &self,
        _host: &str,
        request_id: RequestId,
        auth_token: AuthenticationToken,
        udf_path: UdfPath,
        args: Vec<JsonValue>,
        caller: FunctionCaller,
        ts: ExecuteQueryTimestamp,
        journal: Option<SerializedQueryJournal>,
    ) -> anyhow::Result<RedactedQueryReturn> {
        anyhow::ensure!(
            caller.allowed_visibility() == AllowedVisibility::PublicOnly,
            "This method should not be used by internal callers."
        );

        let validate_time = self.runtime().system_time();
        let identity = self.authenticate(auth_token, validate_time).await?;

        let ts = match ts {
            ExecuteQueryTimestamp::Latest => *self.now_ts_for_reads(),
            ExecuteQueryTimestamp::At(ts) => ts,
        };
        let path = ComponentFunctionPath {
            component: ComponentPath::root(),
            udf_path,
        };
        self.read_only_udf_at_ts(request_id, path, args, identity, ts, journal, caller)
            .await
    }

    async fn execute_public_mutation(
        &self,
        _host: &str,
        request_id: RequestId,
        auth_token: AuthenticationToken,
        udf_path: UdfPath,
        args: Vec<JsonValue>,
        caller: FunctionCaller,
        // Identifier used to make this mutation idempotent.
        mutation_identifier: Option<SessionRequestIdentifier>,
    ) -> anyhow::Result<Result<RedactedMutationReturn, RedactedMutationError>> {
        anyhow::ensure!(
            caller.allowed_visibility() == AllowedVisibility::PublicOnly,
            "This method should not be used by internal callers."
        );

        let validate_time = self.runtime().system_time();
        let identity = self.authenticate(auth_token, validate_time).await?;

        let path = ComponentFunctionPath {
            component: ComponentPath::root(),
            udf_path,
        };
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
        _host: &str,
        request_id: RequestId,
        auth_token: AuthenticationToken,
        udf_path: UdfPath,
        args: Vec<JsonValue>,
        caller: FunctionCaller,
    ) -> anyhow::Result<Result<RedactedActionReturn, RedactedActionError>> {
        anyhow::ensure!(
            caller.allowed_visibility() == AllowedVisibility::PublicOnly,
            "This method should not be used by internal callers."
        );

        let validate_time = self.runtime().system_time();
        let identity = self.authenticate(auth_token, validate_time).await?;

        let path = ComponentFunctionPath {
            component: ComponentPath::root(),
            udf_path,
        };
        self.action_udf(request_id, path, args, identity, caller)
            .await
    }

    async fn latest_timestamp(
        &self,
        _host: &str,
        _request_id: RequestId,
    ) -> anyhow::Result<RepeatableTimestamp> {
        Ok(self.now_ts_for_reads())
    }

    async fn subscribe(&self, token: Token) -> anyhow::Result<Box<dyn SubscriptionTrait>> {
        let inner = self.subscribe(token.clone()).await?;
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

    // Returns true if the subscription validity can be extended to new_ts.
    async fn extend_validity(&mut self, new_ts: Timestamp) -> anyhow::Result<bool>;
}

struct ApplicationSubscription {
    inner: Subscription,
    log: LogReader,

    reads: ReadSet,
    initial_ts: Timestamp,
}

#[async_trait]
impl SubscriptionTrait for ApplicationSubscription {
    fn wait_for_invalidation(&self) -> BoxFuture<'static, anyhow::Result<()>> {
        self.inner.wait_for_invalidation().map(Ok).boxed()
    }

    async fn extend_validity(&mut self, new_ts: Timestamp) -> anyhow::Result<bool> {
        if new_ts < self.initial_ts {
            // new_ts is before the initial subscription timestamp.
            return Ok(false);
        }

        let Some(current_ts) = self.inner.current_ts() else {
            // Subscription no longer valid.
            return Ok(false);
        };

        let current_token = Token::new(self.reads.clone(), current_ts);
        let Some(_new_token) = self.log.refresh_token(current_token, new_ts)? else {
            // Subscription validity can't be extended.
            return Ok(false);
        };

        return Ok(true);
    }
}
