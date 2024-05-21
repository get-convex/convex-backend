use async_trait::async_trait;
use common::{
    components::ComponentFunctionPath,
    pause::PauseClient,
    runtime::Runtime,
    types::{
        AllowedVisibility,
        FunctionCaller,
    },
    value::ConvexValue,
    RequestId,
};
use model::session_requests::types::SessionRequestIdentifier;
use serde_json::Value as JsonValue;
use sync_types::AuthenticationToken;

use crate::{
    redaction::{
        RedactedJsError,
        RedactedLogLines,
    },
    Application,
    RedactedActionError,
    RedactedActionReturn,
    RedactedMutationError,
    RedactedMutationReturn,
};

// A trait that abstracts the backend API. It all state and validation logic
// so http routes can be kept thin and stateless. The implementor is also
// responsible for routing the request to the appropriate backend in the hosted
// version of Convex.
#[async_trait]
pub trait ApplicationApi: Send + Sync {
    async fn execute_public_query(
        &self,
        host: Option<String>,
        request_id: RequestId,
        auth_token: AuthenticationToken,
        path: ComponentFunctionPath,
        args: Vec<JsonValue>,
        caller: FunctionCaller,
        // TODO(presley): Replace this with RedactedQueryReturn.
    ) -> anyhow::Result<(Result<ConvexValue, RedactedJsError>, RedactedLogLines)>;

    async fn execute_public_mutation(
        &self,
        host: Option<String>,
        request_id: RequestId,
        auth_token: AuthenticationToken,
        path: ComponentFunctionPath,
        args: Vec<JsonValue>,
        caller: FunctionCaller,
        // Identifier used to make this mutation idempotent.
        mutation_identifier: Option<SessionRequestIdentifier>,
    ) -> anyhow::Result<Result<RedactedMutationReturn, RedactedMutationError>>;

    async fn execute_public_action(
        &self,
        host: Option<String>,
        request_id: RequestId,
        auth_token: AuthenticationToken,
        path: ComponentFunctionPath,
        args: Vec<JsonValue>,
        caller: FunctionCaller,
    ) -> anyhow::Result<Result<RedactedActionReturn, RedactedActionError>>;
}

// Implements ApplicationApi via Application.
#[async_trait]
impl<RT: Runtime> ApplicationApi for Application<RT> {
    async fn execute_public_query(
        &self,
        _host: Option<String>,
        request_id: RequestId,
        auth_token: AuthenticationToken,
        path: ComponentFunctionPath,
        args: Vec<JsonValue>,
        caller: FunctionCaller,
    ) -> anyhow::Result<(Result<ConvexValue, RedactedJsError>, RedactedLogLines)> {
        anyhow::ensure!(
            caller.allowed_visibility() == AllowedVisibility::PublicOnly,
            "This method should not be used by internal callers."
        );

        let validate_time = self.runtime().system_time();
        let identity = self.authenticate(auth_token, validate_time).await?;

        let ts = *self.now_ts_for_reads();
        let journal = None;

        let query_return = self
            .read_only_udf_at_ts(request_id, path, args, identity, ts, journal, caller)
            .await?;

        Ok((query_return.result, query_return.log_lines))
    }

    async fn execute_public_mutation(
        &self,
        _host: Option<String>,
        request_id: RequestId,
        auth_token: AuthenticationToken,
        path: ComponentFunctionPath,
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
        _host: Option<String>,
        request_id: RequestId,
        auth_token: AuthenticationToken,
        path: ComponentFunctionPath,
        args: Vec<JsonValue>,
        caller: FunctionCaller,
    ) -> anyhow::Result<Result<RedactedActionReturn, RedactedActionError>> {
        anyhow::ensure!(
            caller.allowed_visibility() == AllowedVisibility::PublicOnly,
            "This method should not be used by internal callers."
        );

        let validate_time = self.runtime().system_time();
        let identity = self.authenticate(auth_token, validate_time).await?;

        self.action_udf(request_id, path, args, identity, caller)
            .await
    }
}
