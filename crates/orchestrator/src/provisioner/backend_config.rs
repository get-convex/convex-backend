//! Request-scoped backend infrastructure choices.

use std::{collections::BTreeMap, str::FromStr};

pub const PROVISIONING_MODE_OVERRIDE_KEY: &str = "CONVEX_ORCHESTRATOR_PROVISIONING_MODE";
pub const DATABASE_MODE_OVERRIDE_KEY: &str = "CONVEX_ORCHESTRATOR_DATABASE_MODE";
pub const STORAGE_MODE_OVERRIDE_KEY: &str = "CONVEX_ORCHESTRATOR_STORAGE_MODE";

const SIDECAR_DATABASE_ENV_KEYS: &[&str] = &["POSTGRES_URL"];
const SIDECAR_STORAGE_ENV_KEYS: &[&str] = &[
    "AWS_REGION",
    "AWS_ENDPOINT_URL_S3",
    "AWS_S3_FORCE_PATH_STYLE",
    "AWS_S3_DISABLE_SSE",
    "AWS_S3_DISABLE_CHECKSUMS",
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "S3_STORAGE_EXPORTS_BUCKET",
    "S3_STORAGE_SNAPSHOT_IMPORTS_BUCKET",
    "S3_STORAGE_MODULES_BUCKET",
    "S3_STORAGE_FILES_BUCKET",
    "S3_STORAGE_SEARCH_BUCKET",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvisioningMode {
    Default,
    VolumeSqlite,
    Sidecar,
}

impl Default for ProvisioningMode {
    fn default() -> Self {
        Self::Default
    }
}

impl FromStr for ProvisioningMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "default" => Ok(Self::Default),
            "volume-sqlite" => Ok(Self::VolumeSqlite),
            "sidecar" => Ok(Self::Sidecar),
            other => anyhow::bail!(
                "unknown provisioning mode {other:?} (expected `default`, `volume-sqlite`, or \
                 `sidecar`)"
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseMode {
    Default,
    Sqlite,
    Sidecar,
    External,
}

impl Default for DatabaseMode {
    fn default() -> Self {
        Self::Default
    }
}

impl FromStr for DatabaseMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "default" => Ok(Self::Default),
            "sqlite" => Ok(Self::Sqlite),
            "sidecar" => Ok(Self::Sidecar),
            "external" => Ok(Self::External),
            other => anyhow::bail!(
                "unknown database mode {other:?} (expected `default`, `sqlite`, `sidecar`, or \
                 `external`)"
            ),
        }
    }
}

impl DatabaseMode {
    fn derived_from_provisioning_mode(mode: ProvisioningMode) -> Self {
        match mode {
            ProvisioningMode::Default => Self::Default,
            ProvisioningMode::VolumeSqlite => Self::Sqlite,
            ProvisioningMode::Sidecar => Self::Sidecar,
        }
    }

    fn resolve(
        self,
        provisioning_mode: ProvisioningMode,
        default: &super::ProvisioningStrategy,
    ) -> ResolvedDatabaseMode {
        let mode = if self == Self::Default {
            Self::derived_from_provisioning_mode(provisioning_mode)
        } else {
            self
        };
        match mode {
            Self::Default => match default {
                super::ProvisioningStrategy::Sidecar { .. } => ResolvedDatabaseMode::Sidecar,
                super::ProvisioningStrategy::VolumeSqlite => ResolvedDatabaseMode::Sqlite,
            },
            Self::Sqlite => ResolvedDatabaseMode::Sqlite,
            Self::External => ResolvedDatabaseMode::External,
            Self::Sidecar => match default {
                super::ProvisioningStrategy::Sidecar { .. } => ResolvedDatabaseMode::Sidecar,
                super::ProvisioningStrategy::VolumeSqlite => ResolvedDatabaseMode::Sqlite,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageMode {
    Default,
    Local,
    Sidecar,
    External,
}

impl Default for StorageMode {
    fn default() -> Self {
        Self::Default
    }
}

impl FromStr for StorageMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "default" => Ok(Self::Default),
            "local" => Ok(Self::Local),
            "sidecar" => Ok(Self::Sidecar),
            "external" => Ok(Self::External),
            other => anyhow::bail!(
                "unknown storage mode {other:?} (expected `default`, `local`, `sidecar`, or \
                 `external`)"
            ),
        }
    }
}

impl StorageMode {
    fn derived_from_provisioning_mode(mode: ProvisioningMode) -> Self {
        match mode {
            ProvisioningMode::Default => Self::Default,
            ProvisioningMode::VolumeSqlite => Self::Local,
            ProvisioningMode::Sidecar => Self::Sidecar,
        }
    }

    fn resolve(
        self,
        provisioning_mode: ProvisioningMode,
        default: &super::ProvisioningStrategy,
    ) -> ResolvedStorageMode {
        let mode = if self == Self::Default {
            Self::derived_from_provisioning_mode(provisioning_mode)
        } else {
            self
        };
        match mode {
            Self::Default => match default {
                super::ProvisioningStrategy::Sidecar { .. } => ResolvedStorageMode::Sidecar,
                super::ProvisioningStrategy::VolumeSqlite => ResolvedStorageMode::Local,
            },
            Self::Local => ResolvedStorageMode::Local,
            Self::External => ResolvedStorageMode::External,
            Self::Sidecar => match default {
                super::ProvisioningStrategy::Sidecar { .. } => ResolvedStorageMode::Sidecar,
                super::ProvisioningStrategy::VolumeSqlite => ResolvedStorageMode::Local,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendInfrastructurePlan {
    pub provisioning_mode: ProvisioningMode,
    pub database_mode: DatabaseMode,
    pub storage_mode: StorageMode,
}

impl Default for BackendInfrastructurePlan {
    fn default() -> Self {
        Self {
            provisioning_mode: ProvisioningMode::Default,
            database_mode: DatabaseMode::Default,
            storage_mode: StorageMode::Default,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedDatabaseMode {
    Sqlite,
    Sidecar,
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedStorageMode {
    Local,
    Sidecar,
    External,
}

impl BackendInfrastructurePlan {
    pub fn resolved_database(self, default: &super::ProvisioningStrategy) -> ResolvedDatabaseMode {
        self.database_mode.resolve(self.provisioning_mode, default)
    }

    pub fn resolved_storage(self, default: &super::ProvisioningStrategy) -> ResolvedStorageMode {
        self.storage_mode.resolve(self.provisioning_mode, default)
    }

    pub fn needs_sidecars(self, default: &super::ProvisioningStrategy) -> bool {
        matches!(
            self.resolved_database(default),
            ResolvedDatabaseMode::Sidecar
        ) || matches!(self.resolved_storage(default), ResolvedStorageMode::Sidecar)
    }

    pub fn needs_backend_volume(self, default: &super::ProvisioningStrategy) -> bool {
        matches!(
            self.resolved_database(default),
            ResolvedDatabaseMode::Sqlite
        ) || matches!(self.resolved_storage(default), ResolvedStorageMode::Local)
    }

    pub fn filter_sidecar_env(
        self,
        default: &super::ProvisioningStrategy,
        env: Vec<(&'static str, String)>,
    ) -> Vec<(&'static str, String)> {
        let include_database = matches!(
            self.resolved_database(default),
            ResolvedDatabaseMode::Sidecar
        );
        let include_storage =
            matches!(self.resolved_storage(default), ResolvedStorageMode::Sidecar);

        env.into_iter()
            .filter(|(k, _)| {
                (include_database && SIDECAR_DATABASE_ENV_KEYS.contains(k))
                    || (include_storage && SIDECAR_STORAGE_ENV_KEYS.contains(k))
            })
            .collect()
    }
}

impl ProvisioningMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::VolumeSqlite => "volume-sqlite",
            Self::Sidecar => "sidecar",
        }
    }
}

pub fn backend_infrastructure_from_overrides(
    overrides: &BTreeMap<String, String>,
    explicit: Option<&str>,
) -> anyhow::Result<BackendInfrastructurePlan> {
    let provisioning_mode = match explicit.or_else(|| {
        overrides
            .get(PROVISIONING_MODE_OVERRIDE_KEY)
            .map(String::as_str)
    }) {
        Some(raw) => raw.parse(),
        None => Ok(ProvisioningMode::Default),
    }?;
    let database_mode = match overrides
        .get(DATABASE_MODE_OVERRIDE_KEY)
        .map(String::as_str)
    {
        Some(raw) => raw.parse(),
        None => Ok(DatabaseMode::Default),
    }?;
    let storage_mode = match overrides.get(STORAGE_MODE_OVERRIDE_KEY).map(String::as_str) {
        Some(raw) => raw.parse(),
        None => Ok(StorageMode::Default),
    }?;
    Ok(BackendInfrastructurePlan {
        provisioning_mode,
        database_mode,
        storage_mode,
    })
}

pub fn backend_env_overrides(overrides: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    overrides
        .iter()
        .filter(|(k, _)| {
            !matches!(
                k.as_str(),
                PROVISIONING_MODE_OVERRIDE_KEY
                    | DATABASE_MODE_OVERRIDE_KEY
                    | STORAGE_MODE_OVERRIDE_KEY
            )
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

pub fn upsert_provisioning_mode_override(
    overrides: &mut BTreeMap<String, String>,
    mode: ProvisioningMode,
) {
    if mode == ProvisioningMode::Default {
        overrides.remove(PROVISIONING_MODE_OVERRIDE_KEY);
    } else {
        overrides.insert(
            PROVISIONING_MODE_OVERRIDE_KEY.to_string(),
            mode.as_str().to_string(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_sqlite_mode_overrides_sidecar_default() {
        let default = super::super::ProvisioningStrategy::Sidecar {
            postgres_image: "postgres:16".into(),
            minio_image: "minio:latest".into(),
        };
        let plan = BackendInfrastructurePlan {
            provisioning_mode: ProvisioningMode::VolumeSqlite,
            ..Default::default()
        };

        assert_eq!(
            plan.resolved_database(&default),
            ResolvedDatabaseMode::Sqlite
        );
        assert_eq!(plan.resolved_storage(&default), ResolvedStorageMode::Local);
        assert!(plan.needs_backend_volume(&default));
    }

    #[test]
    fn default_mode_preserves_global_strategy() {
        let default = super::super::ProvisioningStrategy::Sidecar {
            postgres_image: "postgres:16".into(),
            minio_image: "minio:latest".into(),
        };
        let plan = BackendInfrastructurePlan::default();

        assert_eq!(
            plan.resolved_database(&default),
            ResolvedDatabaseMode::Sidecar
        );
        assert_eq!(
            plan.resolved_storage(&default),
            ResolvedStorageMode::Sidecar
        );
        assert!(plan.needs_sidecars(&default));
        assert!(!plan.needs_backend_volume(&default));
    }

    #[test]
    fn sidecar_mode_cannot_invent_sidecars_when_disabled_globally() {
        let default = super::super::ProvisioningStrategy::VolumeSqlite;
        let plan = BackendInfrastructurePlan {
            provisioning_mode: ProvisioningMode::Sidecar,
            ..Default::default()
        };

        assert_eq!(
            plan.resolved_database(&default),
            ResolvedDatabaseMode::Sqlite
        );
        assert_eq!(plan.resolved_storage(&default), ResolvedStorageMode::Local);
        assert!(!plan.needs_sidecars(&default));
        assert!(plan.needs_backend_volume(&default));
    }

    #[test]
    fn mode_can_be_persisted_in_override_map_without_becoming_backend_env() {
        let overrides = BTreeMap::from([
            (
                PROVISIONING_MODE_OVERRIDE_KEY.to_string(),
                "volume-sqlite".to_string(),
            ),
            (
                DATABASE_MODE_OVERRIDE_KEY.to_string(),
                "external".to_string(),
            ),
            (STORAGE_MODE_OVERRIDE_KEY.to_string(), "local".to_string()),
            (
                "MYSQL_URL".to_string(),
                "mysql://user:pass@db:3306".to_string(),
            ),
        ]);
        let plan = backend_infrastructure_from_overrides(&overrides, None).unwrap();

        assert_eq!(plan.provisioning_mode, ProvisioningMode::VolumeSqlite);
        assert_eq!(plan.database_mode, DatabaseMode::External);
        assert_eq!(plan.storage_mode, StorageMode::Local);
        assert_eq!(
            backend_env_overrides(&overrides),
            BTreeMap::from([(
                "MYSQL_URL".to_string(),
                "mysql://user:pass@db:3306".to_string(),
            )]),
        );
    }

    #[test]
    fn sidecar_env_can_be_filtered_by_component() {
        let default = super::super::ProvisioningStrategy::Sidecar {
            postgres_image: "postgres:16".into(),
            minio_image: "minio:latest".into(),
        };
        let plan = BackendInfrastructurePlan {
            provisioning_mode: ProvisioningMode::Sidecar,
            database_mode: DatabaseMode::External,
            storage_mode: StorageMode::Sidecar,
        };

        let filtered = plan.filter_sidecar_env(
            &default,
            vec![
                ("POSTGRES_URL", "postgres://sidecar".to_string()),
                ("AWS_REGION", "us-east-1".to_string()),
                ("S3_STORAGE_FILES_BUCKET", "convex-files".to_string()),
            ],
        );

        assert_eq!(
            filtered,
            vec![
                ("AWS_REGION", "us-east-1".to_string()),
                ("S3_STORAGE_FILES_BUCKET", "convex-files".to_string()),
            ],
        );
    }
}
