use std::{
    fmt::{
        self,
        Display,
    },
    str::FromStr,
    sync::LazyLock,
};

use anyhow::Context;
use cmd_util::env::env_config;
use errors::ErrorMetadata;
pub use metrics::SERVER_VERSION_STR;
#[cfg(any(test, feature = "testing"))]
use proptest::strategy::Strategy;
pub use semver::Version;
use serde::{
    Deserialize,
    Serialize,
};
use tuple_struct::tuple_struct_string;
use value::export::ValueFormat;

// Threshold for each of our clients
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VersionThreshold {
    pub upgrade_required: Version,
    pub unsupported: Version,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeprecationThreshold {
    pub npm: VersionThreshold,
    pub python: VersionThreshold,
    pub rust: VersionThreshold,
}

pub static DEPRECATION_THRESHOLD: LazyLock<DeprecationThreshold> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../deprecation.json"))
        .expect("Couldn't parse deprecation.json")
});

// Enabled in 1.7.0 but we use 1.6.1000 to allow for pre-releases to have this
// feature enabled
pub static MIN_NPM_VERSION_FOR_FUZZY_SEARCH: LazyLock<Version> =
    LazyLock::new(|| env_config("MIN_NPM_VERSION_FOR_FUZZY_SEARCH", Version::new(1, 6, 1000)));

tuple_struct_string!(BackendVersion);

#[derive(Debug, Serialize, PartialEq, Eq)]
pub enum ClientVersionState {
    Unsupported(String),
    // This version is deprecated and will be unsupported soon
    UpgradeRequired(String),
    Supported,
}

impl ClientVersionState {
    pub fn variant_name(&self) -> &str {
        match self {
            ClientVersionState::Unsupported(_) => "Unsupported",
            // NOTE: The string "UpgradeCritical" causes old CLI versions
            // <=1.4.1 to throw a bad error. So we send a different string
            // that makes all CLI versions just print the warning.
            ClientVersionState::UpgradeRequired(_) => "UpgradeRequired",
            ClientVersionState::Supported => "Supported",
        }
    }
}

#[derive(PartialEq, Eq, Clone, Ord, PartialOrd)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct ClientVersion {
    client: ClientType,
    version: ClientVersionIdent,
}

impl From<ClientVersion> for pb::common::ClientVersion {
    fn from(ClientVersion { client, version }: ClientVersion) -> Self {
        Self {
            client: Some(client.to_string()),
            version: Some(version.into()),
        }
    }
}

impl TryFrom<pb::common::ClientVersion> for ClientVersion {
    type Error = anyhow::Error;

    fn try_from(
        pb::common::ClientVersion { client, version }: pb::common::ClientVersion,
    ) -> anyhow::Result<Self> {
        let client = client.context("Missing `client` field")?.parse()?;
        let version = version.context("Missing `version` field")?.try_into()?;
        Ok(ClientVersion { client, version })
    }
}

#[derive(PartialEq, Eq, Clone, Ord, PartialOrd, Debug)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum ClientVersionIdent {
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "(0..5, 0..100, 0..4).prop_map(|(major, minor, patch)| \
                             ClientVersionIdent::Semver(format!(\"{major}.{minor}.{patch}\").\
                             parse().unwrap()))")
    )]
    Semver(Version),
    Unrecognized(String),
}

impl Display for ClientVersionIdent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Semver(v) => write!(f, "{v}"),
            Self::Unrecognized(ident) => write!(f, "{ident}"),
        }
    }
}

impl ClientVersionIdent {
    fn below_threshold(&self, threshold: &Version) -> bool {
        match self {
            Self::Semver(version) => version <= threshold,
            Self::Unrecognized(_) => true,
        }
    }

    fn above_threshold(&self, threshold: &Version) -> bool {
        match self {
            Self::Semver(version) => version >= threshold,
            Self::Unrecognized(_) => true,
        }
    }
}

impl From<ClientVersionIdent> for pb::common::ClientVersionIdent {
    fn from(value: ClientVersionIdent) -> Self {
        let version = match value {
            ClientVersionIdent::Semver(version) => {
                pb::common::client_version_ident::Version::Semver(version.to_string())
            },
            ClientVersionIdent::Unrecognized(str) => {
                pb::common::client_version_ident::Version::Unrecognized(str)
            },
        };
        pb::common::ClientVersionIdent {
            version: Some(version),
        }
    }
}

impl TryFrom<pb::common::ClientVersionIdent> for ClientVersionIdent {
    type Error = anyhow::Error;

    fn try_from(msg: pb::common::ClientVersionIdent) -> anyhow::Result<Self> {
        let version = match msg.version {
            Some(pb::common::client_version_ident::Version::Semver(version)) => {
                ClientVersionIdent::Semver(version.parse()?)
            },
            Some(pb::common::client_version_ident::Version::Unrecognized(str)) => {
                ClientVersionIdent::Unrecognized(str)
            },
            None => anyhow::bail!("Missing `version` field"),
        };
        Ok(version)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum ClientType {
    Python,
    CLI,
    NPM,
    // Actions running in node call into queries/mutations/etc. with this client.
    Actions,
    Rust,
    StreamingImport,
    AirbyteExport,
    FivetranImport,
    FivetranExport,
    // For HTTP requests from the dashboard. Requests from the dashboard via a
    // Convex client will have an `NPM` version
    Dashboard,
    Swift,
    Kotlin,
    // We convert to lower case when we parse, so lets just generate lowercase strings.
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "proptest::string::string_regex(\"[a-z]+\").unwrap().\
                             prop_map(ClientType::Unrecognized)")
    )]
    Unrecognized(String),
}

impl FromStr for ClientType {
    type Err = !;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let client_type = match &*s.to_ascii_lowercase() {
            "python-convex" => Self::Python,
            "python" => Self::Python,
            "npm-cli" => Self::CLI,
            "npm" => Self::NPM,
            "actions" => Self::Actions,
            "rust" => Self::Rust,
            "streaming-import" => Self::StreamingImport,
            "airbyte-export" => Self::AirbyteExport,
            "fivetran-import" => Self::FivetranImport,
            "fivetran-export" => Self::FivetranExport,
            "dashboard" => Self::Dashboard,
            "swift" => Self::Swift,
            "kotlin" => Self::Kotlin,
            unrecognized => Self::Unrecognized(unrecognized.to_string()),
        };
        Ok(client_type)
    }
}

impl Display for ClientType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Python => write!(f, "python"),
            Self::CLI => write!(f, "npm-cli"),
            Self::NPM => write!(f, "npm"),
            Self::Actions => write!(f, "actions"),
            Self::Rust => write!(f, "rust"),
            Self::StreamingImport => write!(f, "streaming-import"),
            Self::AirbyteExport => write!(f, "airbyte-export"),
            Self::FivetranImport => write!(f, "fivetran-import"),
            Self::FivetranExport => write!(f, "fivetran-export"),
            Self::Dashboard => write!(f, "dashboard"),
            Self::Swift => write!(f, "swift"),
            Self::Kotlin => write!(f, "kotlin"),
            Self::Unrecognized(other_client) => write!(f, "{other_client}"),
        }
    }
}

impl ClientType {
    fn upgrade_required_threshold(&self) -> Option<Version> {
        let DeprecationThreshold { npm, python, rust } = &*DEPRECATION_THRESHOLD;
        match self {
            Self::NPM | Self::CLI | Self::Actions => Some(npm.upgrade_required.clone()),
            Self::Python => Some(python.upgrade_required.clone()),
            Self::Rust => Some(rust.upgrade_required.clone()),
            Self::StreamingImport
            | Self::AirbyteExport
            | Self::FivetranImport
            | Self::FivetranExport
            | Self::Dashboard
            | Self::Swift
            | Self::Kotlin
            | Self::Unrecognized(_) => None,
        }
    }

    fn unsupported_threshold(&self) -> Option<Version> {
        let DeprecationThreshold { npm, python, rust } = &*DEPRECATION_THRESHOLD;
        match self {
            Self::NPM | Self::CLI | Self::Actions => Some(npm.unsupported.clone()),
            Self::Python => Some(python.unsupported.clone()),
            Self::Rust => Some(rust.unsupported.clone()),
            Self::StreamingImport
            | Self::AirbyteExport
            | Self::FivetranImport
            | Self::FivetranExport
            | Self::Dashboard
            | Self::Swift
            | Self::Kotlin
            | Self::Unrecognized(_) => None,
        }
    }

    fn upgrade_instructions(&self) -> &str {
        match self {
            Self::NPM | Self::CLI | Self::Actions => {
                "Update your Convex npm version with `npx convex update` or `npm update`."
            },
            Self::Python => {
                "Update your Convex python version with `pip install --upgrade convex`."
            },
            Self::Rust => {
                "Update your Convex rust version with `cargo update -p convex` or by updating \
                 `Cargo.toml`."
            },
            Self::StreamingImport
            | Self::AirbyteExport
            | Self::FivetranImport
            | Self::FivetranExport
            | Self::Dashboard
            | Self::Swift
            | Self::Kotlin
            | Self::Unrecognized(_) => "",
        }
    }
}

impl FromStr for ClientVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        // Use the longest parseable semver spec from the right.
        let parts: Vec<&str> = s.split('-').collect();

        if parts.len() < 2 {
            anyhow::bail!(ErrorMetadata::bad_request(
                "InvalidVersion",
                format!(
                    "Failed to parse client version string: '{s}'. Expected format is \
                     {{client_name}}-{{semver}}, e.g. my-esolang-client-0.0.1"
                ),
            ));
        }

        let split = (1..parts.len()).find_map(|n| {
            Version::parse(parts[n..parts.len()].join("-").as_str())
                .map(|v| (n, v))
                .ok()
        });
        let (client_str, version) = match split {
            Some((n, version)) => {
                let client_str = parts[0..n].join("-");
                (client_str, ClientVersionIdent::Semver(version))
            },
            None => {
                let client_str = parts[0].to_string();
                let version = ClientVersionIdent::Unrecognized(parts[1..].join("-"));
                (client_str, version)
            },
        };
        let Ok(client) = client_str.parse::<ClientType>() else {
            unreachable!()
        };
        Ok(Self { client, version })
    }
}

impl ClientVersion {
    pub fn client(&self) -> &ClientType {
        &self.client
    }

    pub fn version(&self) -> &ClientVersionIdent {
        &self.version
    }

    pub fn current_state(&self) -> ClientVersionState {
        let client = &self.client;
        let version = &self.version;
        let upgrade_instructions = self.client.upgrade_instructions();

        if let Some(unsupported_threshold) = self.client.unsupported_threshold()
            && self.version.below_threshold(&unsupported_threshold)
        {
            return ClientVersionState::Unsupported(format!(
                "The Convex {client} package at version {version} is no longer supported. \
                 {upgrade_instructions}"
            ));
        }
        if let Some(upgrade_required_threshold) = self.client.upgrade_required_threshold()
            && self.version.below_threshold(&upgrade_required_threshold)
        {
            return ClientVersionState::UpgradeRequired(format!(
                "The Convex {client} package at {version} is deprecated and will no longer be \
                 supported soon. When this version is no longer supported, requests to Convex \
                 will fail, so it is best to upgrade and redeploy your application as soon as \
                 possible. {upgrade_instructions}",
            ));
        }

        ClientVersionState::Supported
    }

    // FIXME: remove this From impl once all clients using version params are
    // deprecated.
    pub fn from_path_param(v: Version, path: &str) -> ClientVersion {
        Self {
            client: if path.ends_with("/sync") || path.ends_with("/udf") {
                ClientType::NPM
            } else {
                ClientType::CLI
            },
            version: ClientVersionIdent::Semver(v),
        }
    }

    pub fn unknown() -> ClientVersion {
        Self {
            client: ClientType::Unrecognized("unknown".into()),
            version: ClientVersionIdent::Unrecognized("unknownversion".into()),
        }
    }

    /// Returns true if the client version new enough to require a format param.
    /// Python and JS client got explicit about this at some point, but old
    /// clients still implicitly need the encoded format.
    pub fn default_format(&self) -> ValueFormat {
        let clean_format = match self.client() {
            ClientType::CLI | ClientType::NPM | ClientType::Actions => {
                self.version().above_threshold(&Version::new(1, 4, 1))
            },
            ClientType::Python => self.version().above_threshold(&Version::new(0, 5, 0)),
            ClientType::Rust
            | ClientType::StreamingImport
            | ClientType::AirbyteExport
            | ClientType::FivetranImport
            | ClientType::FivetranExport
            | ClientType::Dashboard
            | ClientType::Swift
            | ClientType::Kotlin
            | ClientType::Unrecognized(_) => true,
        };

        // Old clients use the encoded format by default
        match clean_format {
            true => ValueFormat::ConvexCleanJSON,
            false => ValueFormat::ConvexEncodedJSON,
        }
    }
}

impl fmt::Display for ClientVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.client, self.version)
    }
}

impl fmt::Debug for ClientVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} ({:?})", self, self.current_state())
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;

    use proptest::prelude::*;
    use semver::{
        BuildMetadata,
        Prerelease,
        Version,
    };
    use sync_types::testing::assert_roundtrips;
    use value::export::ValueFormat;

    use super::{
        ClientVersion,
        ClientVersionState,
    };
    use crate::version::{
        ClientType,
        ClientVersionIdent,
        DeprecationThreshold,
        DEPRECATION_THRESHOLD,
    };

    #[test]
    fn test_static_versions() {
        let DeprecationThreshold { npm, python, rust } = &*DEPRECATION_THRESHOLD;
        assert!(npm.upgrade_required >= npm.unsupported);
        assert!(python.upgrade_required >= python.unsupported);
        assert!(rust.upgrade_required >= rust.unsupported);
    }

    #[test]
    fn test_client_version() -> anyhow::Result<()> {
        assert_matches!(
            "npm-cli-0.0.0".parse::<ClientVersion>()?.current_state(),
            ClientVersionState::Unsupported(_)
        );
        let upgrade_required_version_plus_one = Version::new(
            DEPRECATION_THRESHOLD.npm.upgrade_required.major,
            DEPRECATION_THRESHOLD.npm.upgrade_required.minor,
            DEPRECATION_THRESHOLD.npm.upgrade_required.patch + 1,
        );
        let client_version = ClientVersion {
            client: ClientType::NPM,
            version: ClientVersionIdent::Semver(upgrade_required_version_plus_one),
        };
        assert_eq!(
            client_version.current_state(),
            ClientVersionState::Supported
        );

        // Unknown version of NPM are unsupported
        let client_version = ClientVersion {
            client: ClientType::NPM,
            version: ClientVersionIdent::Unrecognized("asdfdsasdf".to_string()),
        };
        assert_matches!(
            client_version.current_state(),
            ClientVersionState::Unsupported(_)
        );

        // Versions higher than what we know about are also considered latest.
        assert_eq!(
            "python-1000.0.0".parse::<ClientVersion>()?.current_state(),
            ClientVersionState::Supported
        );
        assert_eq!(
            "streaming-import-0.0.10".parse::<ClientVersion>()?,
            ClientVersion {
                client: ClientType::StreamingImport,
                version: ClientVersionIdent::Semver(Version::new(0, 0, 10))
            }
        );

        // Not a valid semver
        assert_matches!(
            "npm-1.2.3.4".parse::<ClientVersion>()?.current_state(),
            ClientVersionState::Unsupported(_)
        );

        assert_eq!(
            &format!("{}", "npm-0.0.10".parse::<ClientVersion>()?),
            "npm-0.0.10"
        );
        assert_matches!(
            "npm-0.0.0".parse::<ClientVersion>()?.current_state(),
            ClientVersionState::Unsupported(_),
        );
        assert_eq!(
            &format!("{}", "custom-swift-0.0.10".parse::<ClientVersion>()?),
            "custom-swift-0.0.10"
        );
        assert_eq!(
            &format!(
                "{}",
                "custom-swift-0.0.10-alpha.0".parse::<ClientVersion>()?
            ),
            "custom-swift-0.0.10-alpha.0"
        );
        // longest parseable semver spec from the right
        assert_eq!(
            "some-custom-thing-1.2.3-4.5.6-alpha.7".parse::<ClientVersion>()?,
            ClientVersion {
                client: ClientType::Unrecognized("some-custom-thing".to_string()),
                version: ClientVersionIdent::Semver(Version {
                    major: 1,
                    minor: 2,
                    patch: 3,
                    pre: Prerelease::new("4.5.6-alpha.7")?,
                    build: BuildMetadata::EMPTY
                })
            }
        );
        assert_eq!(
            "big_brain-20240412T160958Z-baea64010a12"
                .parse::<ClientVersion>()?
                .current_state(),
            ClientVersionState::Supported
        );
        Ok(())
    }

    #[test]
    fn test_should_require_format_param() -> anyhow::Result<()> {
        let require = [
            "npm-1.4.1",
            "npm-1.5.0",
            "npm-1.6.0",
            "npm-2.0.0",
            "actions-1.4.1",
            "npm-cli-1.4.1",
            "python-0.5.0",
            "python-0.6.0",
            "python-1.6.0",
            "asdf-0.0.0", // unknown
        ];
        for r in require {
            assert_eq!(
                r.parse::<ClientVersion>()?.default_format(),
                ValueFormat::ConvexCleanJSON,
            );
        }
        let not_require = [
            "npm-1.3.0",
            "npm-1.2.0",
            "npm-0.19.0",
            "actions-1.3.0",
            "npm-cli-1.3.0",
            "python-0.4.0",
            "python-0.3.0",
        ];
        for r in not_require {
            assert_eq!(
                r.parse::<ClientVersion>()?.default_format(),
                ValueFormat::ConvexEncodedJSON,
            );
        }
        Ok(())
    }

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_client_version_roundtrips(u in any::<ClientVersion>()) {
            assert_roundtrips::<ClientVersion, pb::common::ClientVersion>(u);
        }
    }
}
