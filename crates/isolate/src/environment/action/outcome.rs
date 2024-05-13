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
    funrun::ActionOutcome as ActionOutcomeProto,
};
#[cfg(any(test, feature = "testing"))]
use proptest::prelude::*;
use serde_json::Value as JsonValue;
use sync_types::CanonicalizedUdfPath;
use value::ConvexValue;

use super::HttpActionResult;
use crate::{
    environment::helpers::{
        JsonPackedValue,
        SyscallTrace,
    },
    http_action::HttpActionRequestHead,
    ValidatedPathAndArgs,
};

#[derive(Debug, Clone)]
#[cfg_attr(any(test, feature = "testing"), derive(PartialEq))]
pub struct ActionOutcome {
    pub udf_path: CanonicalizedUdfPath,
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
    ) -> anyhow::Result<Self> {
        Ok(ActionOutcome {
            udf_path: path.into_root_udf_path()?,
            arguments,
            identity,
            unix_timestamp: rt.unix_timestamp(),
            result: Err(js_error),
            syscall_trace: SyscallTrace::new(),
            udf_server_version,
        })
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
            udf_path: path.into_root_udf_path()?,
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
            udf_path: _,
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
            any::<CanonicalizedUdfPath>(),
            any::<ConvexArray>(),
            any::<InertIdentity>(),
            any::<UnixTimestamp>(),
            any::<Result<JsonPackedValue, JsError>>(),
            any::<SyscallTrace>(),
        )
            .prop_map(
                |(udf_path, arguments, identity, unix_timestamp, result, syscall_trace)| Self {
                    udf_path,
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
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::{
        ActionOutcome,
        ActionOutcomeProto,
        ValidatedPathAndArgs,
    };

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_action_udf_outcome_roundtrips(udf_outcome in any::<ActionOutcome>()) {
            let udf_outcome_clone = udf_outcome.clone();
            let udf_path = udf_outcome.udf_path.clone();
            let arguments = udf_outcome.arguments.clone();
            let version = udf_outcome.udf_server_version.clone();
            let identity = udf_outcome_clone.identity.clone();
            let path_and_args = ValidatedPathAndArgs::new_for_tests(
                udf_path,
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
    }
}
