//! The `_udf_config` table has a single row with the global configuration
//! for the UDF runtime.

use anyhow::Context;
#[cfg(any(test, feature = "testing"))]
use common::runtime::Runtime;
use common::runtime::UnixTimestamp;
#[cfg(any(test, feature = "testing"))]
use proptest::{
    arbitrary::Arbitrary,
    strategy::Strategy,
};
#[cfg(any(test, feature = "testing"))]
use rand::Rng;
use semver::Version;
use serde::{
    Deserialize,
    Serialize,
};
use value::codegen_convex_serialization;

#[derive(Debug, Clone)]
#[cfg_attr(any(test, feature = "testing"), derive(PartialEq))]
pub struct UdfConfig {
    /// What is the version of `convex` in a developer's
    /// "package.json" when they push their UDFs? We currently allow this to
    /// go forwards and backwards, but we'll eventually want this to only
    /// move forwards. All of the developers pushing to an instance should be on
    /// the same version.
    pub server_version: Version,
    pub import_phase_rng_seed: [u8; 32],
    pub import_phase_unix_timestamp: UnixTimestamp,
}

#[cfg(any(test, feature = "testing"))]
impl Arbitrary for UdfConfig {
    type Parameters = ();

    type Strategy = impl Strategy<Value = UdfConfig>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;

        (any::<[u8; 32]>(), 0..=i64::MAX).prop_map(|(rng_seed, unix_ts_nanos)| UdfConfig {
            server_version: Version::parse("0.0.0").unwrap(),
            import_phase_rng_seed: rng_seed,
            import_phase_unix_timestamp: UnixTimestamp::from_nanos(unix_ts_nanos as u64),
        })
    }
}

impl UdfConfig {
    #[cfg(any(test, feature = "testing"))]
    pub fn new_for_test<RT: Runtime>(rt: &RT, udf_server_version: Version) -> Self {
        Self {
            server_version: udf_server_version,
            import_phase_rng_seed: rt.rng().random(),
            import_phase_unix_timestamp: rt.unix_timestamp(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedUdfConfig {
    server_version: String,
    #[serde(with = "serde_bytes")]
    import_phase_rng_seed: [u8; 32],
    import_phase_unix_timestamp: i64,
}

impl TryFrom<UdfConfig> for SerializedUdfConfig {
    type Error = anyhow::Error;

    fn try_from(config: UdfConfig) -> anyhow::Result<Self> {
        Ok(SerializedUdfConfig {
            server_version: config.server_version.to_string(),
            import_phase_rng_seed: config.import_phase_rng_seed,
            import_phase_unix_timestamp: config
                .import_phase_unix_timestamp
                .as_nanos()
                .try_into()
                .context("Unix timestamp past 2262")?,
        })
    }
}

impl TryFrom<SerializedUdfConfig> for UdfConfig {
    type Error = anyhow::Error;

    fn try_from(value: SerializedUdfConfig) -> anyhow::Result<Self> {
        let server_version = Version::parse(&value.server_version)?;
        let import_phase_rng_seed = value.import_phase_rng_seed;
        let import_phase_unix_timestamp = {
            let nanos = value.import_phase_unix_timestamp;
            anyhow::ensure!(nanos >= 0, "UnixTimestamp before the unix epoch");
            UnixTimestamp::from_nanos(nanos as u64)
        };
        Ok(Self {
            server_version,
            import_phase_rng_seed,
            import_phase_unix_timestamp,
        })
    }
}

codegen_convex_serialization!(UdfConfig, SerializedUdfConfig);

#[cfg(test)]
mod tests {
    use common::runtime::UnixTimestamp;
    use semver::Version;
    use serde_json::json;
    use value::ConvexObject;

    use crate::udf_config::types::UdfConfig;

    #[test]
    fn test_frozen_obj() {
        assert_eq!(
            UdfConfig::try_from(ConvexObject::try_from(json!({
                "importPhaseRngSeed": {"$bytes": "JycnJycnJycnJycnJycnJycnJycnJycnJycnJycnJyc="},
                "importPhaseUnixTimestamp": {"$integer": "AADITmdtwRs="},
                "serverVersion": "123.456.789",
            })).unwrap())
            .unwrap(),
            UdfConfig {
                server_version: Version::new(123, 456, 789),
                import_phase_rng_seed: [39; 32],
                import_phase_unix_timestamp: UnixTimestamp::from_secs_f64(2000000000.).unwrap(),
            }
        );
    }
}
