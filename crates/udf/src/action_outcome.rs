use std::time::Duration;

use anyhow::Context;
use common::{
    components::CanonicalizedComponentFunctionPath,
    errors::JsError,
    identity::InertIdentity,
    knobs::ISOLATE_MAX_USER_HEAP_SIZE,
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    types::HttpActionRoute,
};
use pb::{
    common::{
        function_result::Result as FunctionResultTypeProto,
        FunctionResult as FunctionResultProto,
    },
    outcome::{
        ActionOutcome as ActionOutcomeProto,
        HttpActionOutcome as HttpActionOutcomeProto,
    },
};
#[cfg(any(test, feature = "testing"))]
use proptest::prelude::*;
use semver::Version;
use sync_types::types::SerializedArgs;
use value::JsonPackedValue;

#[cfg(any(test, feature = "testing"))]
use crate::HttpActionRequest;
use crate::{
    validation::ValidatedPathAndArgs,
    HttpActionRequestHead,
    SyscallTrace,
};

#[derive(Debug, Clone)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(proptest_derive::Arbitrary, PartialEq)
)]
pub enum HttpActionResult {
    Streamed,
    Error(JsError),
}

#[derive(Clone)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(proptest_derive::Arbitrary, Debug, PartialEq)
)]
pub struct ActionOutcome {
    pub path: CanonicalizedComponentFunctionPath,
    pub arguments: SerializedArgs,
    pub identity: InertIdentity,

    pub unix_timestamp: UnixTimestamp,

    pub result: Result<JsonPackedValue, JsError>,
    pub syscall_trace: SyscallTrace,

    #[cfg_attr(any(test, feature = "testing"), proptest(value = "None"))]
    pub udf_server_version: Option<semver::Version>,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(
            strategy = "(0..=i64::MAX as u64, any::<u32>()).prop_map(|(secs, nanos)| \
                        Some(Duration::new(secs, nanos)))"
        )
    )]
    // None if node action
    pub user_execution_time: Option<Duration>,
}

impl ActionOutcome {
    /// Used for synthesizing an outcome when we encounter an error before
    /// reaching the isolate.
    pub fn from_error(
        js_error: JsError,
        path: CanonicalizedComponentFunctionPath,
        arguments: SerializedArgs,
        identity: InertIdentity,
        rt: impl Runtime,
        udf_server_version: Option<semver::Version>,
    ) -> Self {
        ActionOutcome {
            path,
            arguments,
            identity,
            unix_timestamp: rt.unix_timestamp(),
            result: Err(js_error),
            syscall_trace: SyscallTrace::new(),
            udf_server_version,
            user_execution_time: Some(Duration::ZERO),
        }
    }

    pub(crate) fn from_proto(
        ActionOutcomeProto {
            unix_timestamp,
            result,
            syscall_trace,
            user_execution_time,
        }: ActionOutcomeProto,
        path_and_args: ValidatedPathAndArgs,
        identity: InertIdentity,
    ) -> anyhow::Result<Self> {
        let result = result.context("Missing result")?;
        let result = match result.result {
            Some(FunctionResultTypeProto::JsonPackedValue(value)) => {
                Ok(JsonPackedValue::from_network(value)?)
            },
            Some(FunctionResultTypeProto::JsError(js_error)) => Err(js_error.try_into()?),
            None => anyhow::bail!("Missing result"),
        };
        let (path, arguments, udf_server_version) = path_and_args.consume();
        Ok(Self {
            path: path.for_logging(),
            arguments,
            identity,
            unix_timestamp: unix_timestamp
                .context("Missing unix_timestamp")?
                .try_into()?,
            result,
            syscall_trace: syscall_trace.context("Missing syscall_trace")?.try_into()?,
            udf_server_version,
            user_execution_time: user_execution_time.map(|d| d.try_into()).transpose()?,
        })
    }
}

impl TryFrom<ActionOutcome> for ActionOutcomeProto {
    type Error = anyhow::Error;

    fn try_from(
        ActionOutcome {
            path: _,
            arguments: _,
            identity: _,
            unix_timestamp,
            result,
            syscall_trace,
            udf_server_version: _,
            user_execution_time,
        }: ActionOutcome,
    ) -> anyhow::Result<Self> {
        let result = match result {
            Ok(value) => FunctionResultTypeProto::JsonPackedValue(value.as_str().to_string()),
            Err(js_error) => FunctionResultTypeProto::JsError(js_error.try_into()?),
        };
        Ok(Self {
            unix_timestamp: Some(unix_timestamp.into()),
            result: Some(FunctionResultProto {
                result: Some(result),
            }),
            syscall_trace: Some(syscall_trace.try_into()?),
            user_execution_time: user_execution_time.map(|t| t.try_into()).transpose()?,
        })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(proptest_derive::Arbitrary, PartialEq)
)]
pub struct HttpActionOutcome {
    pub route: HttpActionRoute,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "any::<HttpActionRequest>().prop_map(|req| req.head)")
    )]
    pub http_request: HttpActionRequestHead,
    pub identity: InertIdentity,

    pub unix_timestamp: UnixTimestamp,

    pub result: HttpActionResult,
    pub syscall_trace: SyscallTrace,

    #[cfg_attr(any(test, feature = "testing"), proptest(value = "None"))]
    pub udf_server_version: Option<semver::Version>,

    memory_in_mb: u64,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(
            strategy = "(0..=i64::MAX as u64, any::<u32>()).prop_map(|(secs, nanos)| \
                        Some(Duration::new(secs, nanos)))"
        )
    )]
    // TODO(ENG-10204): Make required
    pub user_execution_time: Option<Duration>,
}

impl HttpActionOutcome {
    pub fn new(
        route: Option<HttpActionRoute>,
        http_request_head: HttpActionRequestHead,
        identity: InertIdentity,
        unix_timestamp: UnixTimestamp,
        result: HttpActionResult,
        syscall_trace: Option<SyscallTrace>,
        udf_server_version: Option<semver::Version>,
        user_execution_time: Duration,
    ) -> Self {
        Self {
            route: route.unwrap_or(http_request_head.route_for_failure()),
            http_request: http_request_head,
            identity,
            unix_timestamp,
            result,
            syscall_trace: syscall_trace.unwrap_or_default(),
            udf_server_version,
            memory_in_mb: (*ISOLATE_MAX_USER_HEAP_SIZE / (1 << 20))
                .try_into()
                .unwrap(),
            user_execution_time: Some(user_execution_time),
        }
    }

    pub fn memory_in_mb(&self) -> u64 {
        self.memory_in_mb
    }

    pub(crate) fn from_proto(
        HttpActionOutcomeProto {
            unix_timestamp,
            result,
            syscall_trace,
            memory_in_mb,
            path,
            method,
            user_execution_time,
        }: HttpActionOutcomeProto,
        http_request: HttpActionRequestHead,
        udf_server_version: Option<Version>,
        identity: InertIdentity,
    ) -> anyhow::Result<Self> {
        let result = result.context("Missing result")?;
        let result = match result.result {
            Some(FunctionResultTypeProto::JsonPackedValue(_)) => {
                anyhow::bail!("Http actions not expected to have aresult")
            },
            Some(FunctionResultTypeProto::JsError(js_error)) => {
                HttpActionResult::Error(js_error.try_into()?)
            },
            None => HttpActionResult::Streamed,
        };
        // TODO: Add `.context()` and remove fallback to `HttpRequestHead`
        let method = match method {
            Some(m) => m.parse()?,
            None => http_request.method.clone().try_into()?,
        };
        let path = match path {
            Some(p) => p,
            None => http_request.url.to_string(),
        };
        Ok(Self {
            identity,
            unix_timestamp: unix_timestamp
                .context("Missing unix_timestamp")?
                .try_into()?,
            result,
            syscall_trace: syscall_trace.context("Missing syscall_trace")?.try_into()?,
            memory_in_mb,
            http_request,
            udf_server_version,
            route: HttpActionRoute { method, path },
            user_execution_time: user_execution_time.map(|d| d.try_into()).transpose()?,
        })
    }
}

impl TryFrom<HttpActionOutcome> for HttpActionOutcomeProto {
    type Error = anyhow::Error;

    fn try_from(
        HttpActionOutcome {
            route,
            http_request: _,
            identity: _,
            unix_timestamp,
            result,
            syscall_trace,
            udf_server_version: _,
            memory_in_mb,
            user_execution_time,
        }: HttpActionOutcome,
    ) -> anyhow::Result<Self> {
        let result = match result {
            HttpActionResult::Streamed => None,
            HttpActionResult::Error(js_error) => {
                Some(FunctionResultTypeProto::JsError(js_error.try_into()?))
            },
        };
        Ok(Self {
            unix_timestamp: Some(unix_timestamp.into()),
            result: Some(FunctionResultProto { result }),
            syscall_trace: Some(syscall_trace.try_into()?),
            memory_in_mb,
            path: Some(route.path.to_string()),
            method: Some(route.method.to_string()),
            user_execution_time: user_execution_time.map(|t| t.try_into()).transpose()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use proptest::prelude::*;

    use super::{
        ActionOutcome,
        ActionOutcomeProto,
        HttpActionOutcomeProto,
        ValidatedPathAndArgs,
    };
    use crate::HttpActionOutcome;

    proptest! {
        #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_action_udf_outcome_roundtrips(udf_outcome in any::<ActionOutcome>()) {
            let udf_outcome_clone = udf_outcome.clone();
            let path = udf_outcome.path.clone();
            let arguments = udf_outcome.arguments.clone();
            let version = udf_outcome.udf_server_version.clone();
            let identity = udf_outcome_clone.identity.clone();
            let path_and_args = ValidatedPathAndArgs::new_for_tests_in_component(
                path,
                arguments,
                version
            );
            let proto = ActionOutcomeProto::try_from(udf_outcome_clone).unwrap();
            let udf_outcome_from_proto = ActionOutcome::from_proto(
                proto,
                path_and_args,
                identity
            ).unwrap();
            assert_eq!(udf_outcome, udf_outcome_from_proto);
        }

        #[test]
        fn test_http_action_outcome_roundtrips(udf_outcome in any::<HttpActionOutcome>()) {
            let udf_outcome_clone = udf_outcome.clone();
            let http_request = udf_outcome.http_request.clone();
            let version = udf_outcome.udf_server_version.clone();
            let identity = udf_outcome_clone.identity.clone();
            let proto = HttpActionOutcomeProto::try_from(udf_outcome_clone).unwrap();
            let udf_outcome_from_proto = HttpActionOutcome::from_proto(
                proto,
                http_request,
                version,
                identity,
            ).unwrap();
            assert_eq!(udf_outcome, udf_outcome_from_proto);
        }
    }
}
