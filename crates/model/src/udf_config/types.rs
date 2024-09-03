use std::collections::BTreeMap;

use anyhow::Context;
#[cfg(any(test, feature = "testing"))]
use common::runtime::Runtime;
/// The `_udf_config` table has a single row with the global configuration
/// for the UDF runtime.
use common::{
    obj,
    runtime::UnixTimestamp,
};
#[cfg(any(test, feature = "testing"))]
use rand::Rng;
use semver::Version;
use value::{
    ConvexObject,
    ConvexValue,
};

#[derive(Debug, Clone)]
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

impl UdfConfig {
    #[cfg(any(test, feature = "testing"))]
    pub fn new_for_test<RT: Runtime>(rt: &RT, udf_server_version: Version) -> Self {
        Self {
            server_version: udf_server_version,
            import_phase_rng_seed: rt.rng().gen(),
            import_phase_unix_timestamp: rt.unix_timestamp(),
        }
    }
}

impl TryFrom<UdfConfig> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(config: UdfConfig) -> anyhow::Result<Self> {
        obj! {
            "serverVersion" => format!("{}", config.server_version),
            "importPhaseRngSeed" =>
                config.import_phase_rng_seed.to_vec(),
            "importPhaseUnixTimestamp" => ConvexValue::Int64(
                config.import_phase_unix_timestamp.as_nanos().try_into().context("Unix timestamp past 2262")?
            ),
        }
    }
}

impl TryFrom<ConvexObject> for UdfConfig {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> anyhow::Result<Self> {
        let mut fields: BTreeMap<_, _> = value.into();
        let server_version = match fields.remove("serverVersion") {
            Some(ConvexValue::String(s)) => Version::parse(&s[..])?,
            v => anyhow::bail!("Invalid serverVersion field for UdfConfig: {v:?}"),
        };
        let import_phase_rng_seed = match fields.remove("importPhaseRngSeed") {
            Some(ConvexValue::Bytes(o)) => {
                let Ok(rng_seed) = Vec::from(o).try_into() else {
                    anyhow::bail!(
                        "Invalid importPhaseRngSeed field for UdfConfig must be 32 bytes"
                    );
                };
                rng_seed
            },
            v => anyhow::bail!("Invalid importPhaseRngSeed field for UdfConfig: {:?}", v),
        };
        let import_phase_unix_timestamp = match fields.remove("importPhaseUnixTimestamp") {
            Some(ConvexValue::Int64(nanos)) => {
                anyhow::ensure!(nanos >= 0, "UnixTimestamp before the unix epoch    ");
                UnixTimestamp::from_nanos(nanos as u64)
            },
            v => anyhow::bail!("Invalid importPhaseTimestamp field for UdfConfig: {:?}", v),
        };
        Ok(Self {
            server_version,
            import_phase_rng_seed,
            import_phase_unix_timestamp,
        })
    }
}
