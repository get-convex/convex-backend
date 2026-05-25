//! Build-time-generated catalog of every `env_config` knob defined in
//! `crates/common/src/knobs.rs`, plus a hand-curated overlay
//! (`exposure.rs`) declaring which knobs are "Curated" (shown in the
//! main dialog), "TierTuned" (set by the tier ladder), or "Advanced"
//! (only visible in the full editor).

pub mod exposure;

#[derive(Debug, Clone, Copy)]
pub struct KnobMeta {
    pub env_var: &'static str,
    pub description: &'static str,
    pub category: &'static str,
    pub default_value: Option<&'static str>,
}

/// Build-time-generated. Do not hand-edit `$OUT_DIR/known_knobs.rs`.
pub const KNOWN_KNOBS: &[KnobMeta] = include!(concat!(env!("OUT_DIR"), "/known_knobs.rs"));

pub fn find(env_var: &str) -> Option<&'static KnobMeta> {
    KNOWN_KNOBS.iter().find(|k| k.env_var == env_var)
}

const INFRASTRUCTURE_OVERRIDE_KEYS: &[&str] = &[
    crate::provisioner::backend_config::PROVISIONING_MODE_OVERRIDE_KEY,
    crate::provisioner::backend_config::DATABASE_MODE_OVERRIDE_KEY,
    crate::provisioner::backend_config::STORAGE_MODE_OVERRIDE_KEY,
    "POSTGRES_URL",
    "MYSQL_URL",
    "AWS_REGION",
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_SESSION_TOKEN",
    "S3_ENDPOINT_URL",
    "AWS_S3_FORCE_PATH_STYLE",
    "AWS_S3_DISABLE_SSE",
    "AWS_S3_DISABLE_CHECKSUMS",
    "S3_STORAGE_EXPORTS_BUCKET",
    "S3_STORAGE_SNAPSHOT_IMPORTS_BUCKET",
    "S3_STORAGE_MODULES_BUCKET",
    "S3_STORAGE_FILES_BUCKET",
    "S3_STORAGE_SEARCH_BUCKET",
];

pub fn is_infrastructure_override(env_var: &str) -> bool {
    INFRASTRUCTURE_OVERRIDE_KEYS.contains(&env_var)
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ValidationError {
    #[error("unknown knob env var: {0}")]
    Unknown(String),
}

/// Reject overrides whose env var isn't in the registry. Type-validation
/// (parsing the value against the declared `KnobType`) is added when the
/// curated/exposure layer grows that field; for v1 we keep validation to
/// "is this a known knob".
pub fn validate(env_var: &str, _value: &str) -> Result<(), ValidationError> {
    if find(env_var).is_some() || is_infrastructure_override(env_var) {
        Ok(())
    } else {
        Err(ValidationError::Unknown(env_var.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracted_a_reasonable_number_of_knobs() {
        // common/src/knobs.rs has ~259 entries; if extraction drops below
        // 200 something has broken silently in the build.rs walker.
        assert!(
            KNOWN_KNOBS.len() >= 200,
            "expected ≥200 knobs, got {}",
            KNOWN_KNOBS.len()
        );
    }

    #[test]
    fn well_known_knobs_present() {
        for var in [
            "ACTIONS_USER_TIMEOUT_SECS",
            "DOCUMENT_RETENTION_DELAY",
            "MAX_TRANSACTION_WINDOW_SECONDS",
            "TRANSACTION_MAX_NUM_USER_WRITES",
            "FUNCTION_MAX_ARGS_SIZE",
            "FUNCTION_MAX_RESULT_SIZE",
            "FUNRUN_INDEX_CACHE_SIZE",
            "FUNRUN_MODULE_CACHE_SIZE",
            "FUNRUN_CODE_CACHE_SIZE",
            "UDF_USE_FUNRUN",
            "FUNRUN_MAX_ISOLATE_WORKERS",
            "HTTP_SERVER_MAX_CONCURRENT_REQUESTS",
            "RUNTIME_WORKER_THREADS",
        ] {
            assert!(find(var).is_some(), "registry missing knob {var}");
        }
    }

    #[test]
    fn validate_unknown_rejected() {
        let res = validate("NOT_A_REAL_KNOB", "1");
        assert_eq!(res, Err(ValidationError::Unknown("NOT_A_REAL_KNOB".into())));
    }

    #[test]
    fn validate_known_accepted() {
        assert!(validate("UDF_USE_FUNRUN", "true").is_ok());
    }

    #[test]
    fn extracts_default_values_for_common_knobs() {
        assert_eq!(
            find("UDF_CACHE_MAX_SIZE").and_then(|k| k.default_value),
            Some("104857600"),
        );
        assert_eq!(
            find("UDF_USE_FUNRUN").and_then(|k| k.default_value),
            Some("true"),
        );
        assert_eq!(
            find("FUNCTION_MAX_ARGS_SIZE").and_then(|k| k.default_value),
            Some("16777216"),
        );
    }

    #[test]
    fn validate_infrastructure_overrides_accepted() {
        for var in [
            crate::provisioner::backend_config::PROVISIONING_MODE_OVERRIDE_KEY,
            crate::provisioner::backend_config::DATABASE_MODE_OVERRIDE_KEY,
            crate::provisioner::backend_config::STORAGE_MODE_OVERRIDE_KEY,
            "POSTGRES_URL",
            "MYSQL_URL",
            "S3_ENDPOINT_URL",
            "AWS_S3_FORCE_PATH_STYLE",
            "S3_STORAGE_FILES_BUCKET",
        ] {
            assert!(validate(var, "value").is_ok(), "{var} should be accepted");
        }
    }
}
