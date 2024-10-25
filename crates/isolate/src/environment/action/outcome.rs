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
    value::ConvexArray,
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
use serde_json::Value as JsonValue;
use value::ConvexValue;

use super::HttpActionResult;
#[cfg(any(test, feature = "testing"))]
use crate::HttpActionRequest;
use crate::{
    environment::helpers::{
        JsonPackedValue,
        SyscallTrace,
    },
    http_action::HttpActionRequestHead,
    ValidatedPathAndArgs,
};

#[derive(Clone)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug, PartialEq))]
pub struct ActionOutcome {
    pub path: CanonicalizedComponentFunctionPath,
    pub arguments: ConvexArray,
    pub identity: InertIdentity,

    pub unix_timestamp: UnixTimestamp,

    pub result: Result<JsonPackedValue, JsError>,
    pub syscall_trace: SyscallTrace,

    pub udf_server_version: Option<semver::Version>,
}

impl ActionOutcome {
    /// Used for synthesizing an outcome when we encounter an error before
    /// reaching the isolate.
    pub fn from_error(
        js_error: JsError,
        path: CanonicalizedComponentFunctionPath,
        arguments: ConvexArray,
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
        }
    }

    pub(crate) fn from_proto(
        ActionOutcomeProto {
            unix_timestamp,
            result,
            syscall_trace,
        }: ActionOutcomeProto,
        path_and_args: ValidatedPathAndArgs,
        identity: InertIdentity,
    ) -> anyhow::Result<Self> {
        let result = result.ok_or_else(|| anyhow::anyhow!("Missing result"))?;
        let result = match result.result {
            Some(FunctionResultTypeProto::JsonPackedValue(value)) => {
                let json: JsonValue = serde_json::from_str(&value)?;
                let value = ConvexValue::try_from(json)?;
                Ok(JsonPackedValue::pack(value))
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
        })
    }
}

#[cfg(any(test, feature = "testing"))]
impl Arbitrary for ActionOutcome {
    type Parameters = ();

    type Strategy = impl Strategy<Value = ActionOutcome>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        (
            any::<CanonicalizedComponentFunctionPath>(),
            any::<ConvexArray>(),
            any::<InertIdentity>(),
            any::<UnixTimestamp>(),
            any::<Result<JsonPackedValue, JsError>>(),
            any::<SyscallTrace>(),
        )
            .prop_map(
                |(path, arguments, identity, unix_timestamp, result, syscall_trace)| Self {
                    path,
                    arguments,
                    identity,
                    unix_timestamp,
                    result,
                    syscall_trace,
                    // Ok to not generate semver::Version because it is not serialized anyway
                    udf_server_version: None,
                },
            )
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(any(test, feature = "testing"), derive(PartialEq))]
pub struct HttpActionOutcome {
    pub route: HttpActionRoute,
    pub http_request: HttpActionRequestHead,
    pub identity: InertIdentity,

    pub unix_timestamp: UnixTimestamp,

    pub result: HttpActionResult,
    pub syscall_trace: SyscallTrace,

    pub udf_server_version: Option<semver::Version>,

    memory_in_mb: u64,
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
        }: HttpActionOutcomeProto,
        http_request: HttpActionRequestHead,
        udf_server_version: Option<Version>,
        identity: InertIdentity,
    ) -> anyhow::Result<Self> {
        let result = result.ok_or_else(|| anyhow::anyhow!("Missing result"))?;
        let result = match result.result {
            Some(FunctionResultTypeProto::JsonPackedValue(_)) => {
                anyhow::bail!("Http actions not expected to have aresult")
            },
            Some(FunctionResultTypeProto::JsError(js_error)) => {
                HttpActionResult::Error(js_error.try_into()?)
            },
            None => HttpActionResult::Streamed,
        };
        Ok(Self {
            identity,
            unix_timestamp: unix_timestamp
                .context("Missing unix_timestamp")?
                .try_into()?,
            result,
            syscall_trace: syscall_trace.context("Missing syscall_trace")?.try_into()?,
            memory_in_mb,
            http_request: http_request.clone(),
            udf_server_version,
            route: HttpActionRoute {
                method: http_request.method.try_into()?,
                path: http_request.url.to_string(),
            },
        })
    }
}

impl TryFrom<HttpActionOutcome> for HttpActionOutcomeProto {
    type Error = anyhow::Error;

    fn try_from(
        HttpActionOutcome {
            route: _,
            http_request: _,
            identity: _,
            unix_timestamp,
            result,
            syscall_trace,
            udf_server_version: _,
            memory_in_mb,
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
        })
    }
}

#[cfg(any(test, feature = "testing"))]
impl Arbitrary for HttpActionOutcome {
    type Parameters = ();

    type Strategy = impl Strategy<Value = HttpActionOutcome>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        (
            any::<HttpActionRequest>(),
            any::<HttpActionResult>(),
            any::<InertIdentity>(),
            any::<UnixTimestamp>(),
            any::<SyscallTrace>(),
            any::<u64>(),
        )
            .prop_map(
                |(request, result, identity, unix_timestamp, syscall_trace, memory_in_mb)| Self {
                    http_request: request.head.clone(),
                    result,
                    route: HttpActionRoute {
                        method: request.head.method.try_into().unwrap(),
                        path: request.head.url.to_string(),
                    },
                    identity,
                    unix_timestamp,
                    syscall_trace,
                    memory_in_mb,
                    // Ok to not generate semver::Version because it is not serialized anyway
                    udf_server_version: None,
                },
            )
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
