use anyhow::Context;
use common::{
    components::CanonicalizedComponentFunctionPath,
    errors::JsError,
    identity::InertIdentity,
    log_lines::{
        LogLine,
        LogLines,
    },
    query_journal::QueryJournal,
    runtime::{
        Runtime,
        UnixTimestamp,
    },
    value::ConvexArray,
};
use pb::{
    common::{
        function_result::Result as FunctionResultTypeProto,
        FunctionResult as FunctionResultProto,
    },
    outcome::UdfOutcome as UdfOutcomeProto,
};
#[cfg(any(test, feature = "testing"))]
use proptest::prelude::Arbitrary;
#[cfg(any(test, feature = "testing"))]
use proptest::prelude::Strategy;
use rand::Rng;
use serde_json::Value as JsonValue;
use sync_types::CanonicalizedUdfPath;
use value::{
    heap_size::HeapSize,
    ConvexValue,
};

use crate::{
    environment::helpers::{
        JsonPackedValue,
        SyscallTrace,
    },
    ValidatedPathAndArgs,
};

#[derive(Debug, Clone)]
#[cfg_attr(any(test, feature = "testing"), derive(PartialEq))]
pub struct UdfOutcome {
    pub udf_path: CanonicalizedUdfPath,
    pub arguments: ConvexArray,
    pub identity: InertIdentity,

    pub rng_seed: [u8; 32],
    pub observed_rng: bool,

    pub unix_timestamp: UnixTimestamp,
    pub observed_time: bool,

    pub log_lines: LogLines,
    pub journal: QueryJournal,

    // QueryUdfOutcomes are stored in the Udf level cache, which is why we would like
    // them to have more compact representation.
    pub result: Result<JsonPackedValue, JsError>,

    pub syscall_trace: SyscallTrace,

    pub udf_server_version: Option<semver::Version>,
}

#[cfg(any(test, feature = "testing"))]
impl Arbitrary for UdfOutcome {
    type Parameters = ();

    type Strategy = impl Strategy<Value = UdfOutcome>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        (
            any::<CanonicalizedUdfPath>(),
            any::<ConvexArray>(),
            any::<InertIdentity>(),
            any::<[u8; 32]>(),
            any::<bool>(),
            any::<UnixTimestamp>(),
            any::<bool>(),
            any::<LogLines>(),
            any::<QueryJournal>(),
            any::<Result<JsonPackedValue, JsError>>(),
            any::<SyscallTrace>(),
        )
            .prop_map(
                |(
                    udf_path,
                    arguments,
                    identity,
                    rng_seed,
                    observed_rng,
                    unix_timestamp,
                    observed_time,
                    log_lines,
                    journal,
                    result,
                    syscall_trace,
                )| Self {
                    udf_path,
                    arguments,
                    identity,
                    rng_seed,
                    observed_rng,
                    unix_timestamp,
                    observed_time,
                    log_lines,
                    journal,
                    result,
                    syscall_trace,
                    // Ok to not generate semver::Version because it is not serialized anyway
                    udf_server_version: None,
                },
            )
    }
}

impl HeapSize for UdfOutcome {
    fn heap_size(&self) -> usize {
        self.udf_path.heap_size()
            + self.arguments.heap_size()
            + self.identity.heap_size()
            + self.log_lines.heap_size()
            + self.journal.heap_size()
            + self.result.heap_size()
            + self.syscall_trace.heap_size()
    }
}

impl TryFrom<UdfOutcome> for UdfOutcomeProto {
    type Error = anyhow::Error;

    fn try_from(
        UdfOutcome {
            udf_path: _,
            arguments: _,
            identity: _,
            rng_seed,
            observed_rng,
            unix_timestamp,
            observed_time,
            log_lines,
            journal,
            result,
            syscall_trace,
            udf_server_version: _,
        }: UdfOutcome,
    ) -> anyhow::Result<Self> {
        let result = match result {
            Ok(value) => FunctionResultTypeProto::JsonPackedValue(value.as_str().to_string()),
            Err(js_error) => FunctionResultTypeProto::JsError(js_error.try_into()?),
        };
        Ok(Self {
            rng_seed: Some(rng_seed.to_vec()),
            observed_rng: Some(observed_rng),
            unix_timestamp: Some(unix_timestamp.into()),
            observed_time: Some(observed_time),
            log_lines: log_lines.into_iter().map(|l| l.into()).collect(),
            journal: Some(journal.into()),
            result: Some(FunctionResultProto {
                result: Some(result),
            }),
            syscall_trace: Some(syscall_trace.try_into()?),
        })
    }
}

impl UdfOutcome {
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
        Ok(UdfOutcome {
            udf_path: path.into_root_udf_path()?,
            arguments,
            identity,
            rng_seed: rt.with_rng(|rng| rng.gen()),
            observed_rng: false,
            unix_timestamp: rt.unix_timestamp(),
            observed_time: false,
            log_lines: vec![].into(),
            journal: QueryJournal::new(),
            result: Err(js_error),
            syscall_trace: SyscallTrace::new(),
            udf_server_version,
        })
    }

    pub(crate) fn from_proto(
        UdfOutcomeProto {
            rng_seed,
            observed_rng,
            unix_timestamp,
            observed_time,
            log_lines,
            journal,
            result,
            syscall_trace,
        }: UdfOutcomeProto,
        path_and_args: ValidatedPathAndArgs,
        identity: InertIdentity,
    ) -> anyhow::Result<Self> {
        let rng_seed = rng_seed.ok_or_else(|| anyhow::anyhow!("Missing rng_seed"))?;
        let rng_seed = rng_seed
            .as_slice()
            .try_into()
            .context("Invalid rng_seed length")?;
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
        let log_lines = log_lines.into_iter().map(LogLine::try_from).try_collect()?;
        Ok(Self {
            udf_path: path.into_root_udf_path()?,
            arguments,
            identity,
            rng_seed,
            observed_rng: observed_rng.unwrap_or_default(),
            unix_timestamp: unix_timestamp
                .ok_or_else(|| anyhow::anyhow!("Missing unix_timestamp"))?
                .try_into()?,
            observed_time: observed_time.unwrap_or_default(),
            log_lines,
            journal: journal
                .ok_or_else(|| anyhow::anyhow!("Missing journal"))?
                .try_into()?,
            result,
            syscall_trace: syscall_trace
                .ok_or_else(|| anyhow::anyhow!("Missing syscall_trace"))?
                .try_into()?,
            udf_server_version,
        })
    }
}

#[cfg(test)]
mod tests {
    use common::components::{
        CanonicalizedComponentFunctionPath,
        ComponentId,
    };
    use proptest::prelude::*;

    use super::{
        UdfOutcome,
        UdfOutcomeProto,
        ValidatedPathAndArgs,
    };

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_udf_outcome_roundtrips(udf_outcome in any::<UdfOutcome>()) {
            let udf_outcome_clone = udf_outcome.clone();
            let udf_path = udf_outcome.udf_path.clone();
            let arguments = udf_outcome.arguments.clone();
            let version = udf_outcome.udf_server_version.clone();
            let identity = udf_outcome_clone.identity.clone();
            let path_and_args = ValidatedPathAndArgs::new_for_tests(
                CanonicalizedComponentFunctionPath {
                    component: ComponentId::Root,
                    udf_path,
                },
                arguments,
                version
            );
            let proto = UdfOutcomeProto::try_from(udf_outcome_clone).unwrap();
            let udf_outcome_from_proto = UdfOutcome::from_proto(
                proto,
                path_and_args,
                identity
            ).unwrap();
            assert_eq!(udf_outcome, udf_outcome_from_proto);
        }
    }
}
