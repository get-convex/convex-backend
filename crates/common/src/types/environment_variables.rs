use std::{
    str::FromStr,
    sync::LazyLock,
};

use errors::ErrorMetadata;
use regex::Regex;
use serde::{
    Deserialize,
    Serialize,
};

#[rustfmt::skip]
#[derive(
    Clone, Debug, Eq, PartialEq, PartialOrd, Ord,
    Serialize, Deserialize, derive_more::Display, Hash,
)]
pub struct EnvVarName(String);

impl From<EnvVarName> for String {
    fn from(value: EnvVarName) -> Self {
        value.0
    }
}

#[rustfmt::skip]
#[derive(
    Clone, Debug, Eq, PartialEq, PartialOrd, Ord,
    Serialize, Deserialize, derive_more::Display, Hash,
)]
pub struct EnvVarValue(String);

impl From<EnvVarValue> for String {
    fn from(value: EnvVarValue) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EnvironmentVariable {
    pub name: EnvVarName,
    pub value: EnvVarValue,
}

impl AsRef<str> for EnvVarName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for EnvVarValue {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

static NAME_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z_]+[a-zA-Z0-9_]*$").unwrap());

/// NOTE: Make sure to update the doc if you change any of these limits. Also
/// don't reduce them since that might break existing projects.

/// Maximum number of environment variables that can be stored.
/// Also update client-side limit GenericEnvironmentVariables.tsx.
pub const ENV_VAR_LIMIT: usize = 100;
/// Maximum length of the name of an environment variable
pub const MAX_NAME_LENGTH: usize = 40;
/// Maximum length of an environment variable value. 8KiB corresponds to the
/// maximum length of an HTTP header.
pub const MAX_VALUE_LENGTH: usize = 8 * (1 << 10);

impl FromStr for EnvVarName {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        anyhow::ensure!(
            NAME_REGEX.is_match(s),
            ErrorMetadata::bad_request(
                "EnvironmentVariableNameInvalid",
                format!(
                    "The environment variable name {s} is invalid. Environment variable names \
                     must begin with a letter and may only include characters a-z, A-Z, 0-9, and \
                     underscores."
                ),
            )
        );
        anyhow::ensure!(
            s.len() <= MAX_NAME_LENGTH,
            ErrorMetadata::bad_request(
                "EnvironmentVariableNameTooLong",
                format!(
                    "The environment variable name {s} is too long. Environment variable names \
                     must be less than {MAX_NAME_LENGTH}."
                )
            )
        );
        Ok(Self(s.to_owned()))
    }
}

impl FromStr for EnvVarValue {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let len = s.len();
        anyhow::ensure!(
            len <= MAX_VALUE_LENGTH,
            ErrorMetadata::bad_request(
                "EnvironmentVariableValueTooLarge",
                format!(
                    "The environment variable value is {len} bytes, which is too large. (max \
                     size: {MAX_VALUE_LENGTH}"
                ),
            )
        );
        Ok(Self(s.to_owned()))
    }
}

impl EnvironmentVariable {
    pub fn new(name: EnvVarName, value: EnvVarValue) -> Self {
        Self { name, value }
    }

    pub fn name(&self) -> &EnvVarName {
        &self.name
    }

    pub fn value(&self) -> &EnvVarValue {
        &self.value
    }

    pub fn into_value(self) -> EnvVarValue {
        self.value
    }
}

pub fn env_var_limit_met() -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "EnvVarLimitMet",
        format!("The environment variable limit ({ENV_VAR_LIMIT}) has been met."),
    )
}

pub fn env_var_name_not_unique(name: Option<&EnvVarName>) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "EnvVarNameNotUnique",
        match name {
            Some(n) => format!("An environment variable with name \"{n}\" already exists"),
            None => "One or more environment variable name is not unique".to_string(),
        },
    )
}

pub fn env_var_name_forbidden(name: &EnvVarName) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "EnvVarNameForbidden",
        format!("Environment variable with name \"{name}\" is built-in and cannot be overridden"),
    )
}

#[cfg(any(test, feature = "testing"))]
mod proptest {
    const ENV_VAR_NAME_REGEX: &str = "_[a-zA-Z][a-zA-Z0-9_]{0,38}";
    use std::str::FromStr;

    use proptest::prelude::*;

    use crate::types::{
        EnvVarName,
        EnvVarValue,
        EnvironmentVariable,
    };

    impl proptest::arbitrary::Arbitrary for EnvVarName {
        type Parameters = ();

        type Strategy = impl proptest::strategy::Strategy<Value = EnvVarName>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            ENV_VAR_NAME_REGEX.prop_filter_map("Invalid environment variable name", |s| {
                let name = EnvVarName::from_str(&s);
                name.ok()
            })
        }
    }

    #[cfg(any(test, feature = "testing"))]
    impl proptest::arbitrary::Arbitrary for EnvVarValue {
        type Parameters = ();

        type Strategy = impl proptest::strategy::Strategy<Value = EnvVarValue>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            use proptest::prelude::*;
            any::<String>().prop_filter_map("Invalid environment variable value", |s| {
                EnvVarValue::from_str(&s).ok()
            })
        }
    }

    #[cfg(any(test, feature = "testing"))]
    impl proptest::arbitrary::Arbitrary for EnvironmentVariable {
        type Parameters = ();

        type Strategy = impl proptest::strategy::Strategy<Value = EnvironmentVariable>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            use proptest::prelude::*;
            any::<(EnvVarName, EnvVarValue)>()
                .prop_map(|(name, value)| EnvironmentVariable { name, value })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::{
        from_utf8,
        FromStr,
    };

    use crate::types::{
        environment_variables::MAX_VALUE_LENGTH,
        EnvVarName,
        EnvVarValue,
    };

    #[test]
    fn valid_env_var_name() {
        // Valid
        assert!(EnvVarName::from_str("a_good_env_var_name").is_ok());
        assert!(EnvVarName::from_str("_a_good_env_var_name").is_ok());

        // Invalid
        assert!(EnvVarName::from_str("1_bad_env_var_name").is_err());
        assert!(EnvVarName::from_str("bad_env_var=name").is_err());
        assert!(EnvVarName::from_str("SUPER_LONG_NAME_____________________________________________________________________________").is_err());
        assert!(EnvVarName::from_str("bad_env_var-name").is_err());
    }

    #[test]
    fn valid_env_var_value() {
        // Valid
        assert!(EnvVarValue::from_str(
            "any_wacky!-$sq28@#%^@#!)*&\
             ____________________________________________________________________________"
        )
        .is_ok());

        // Too long
        let v = vec![0; MAX_VALUE_LENGTH + 1];
        let s = from_utf8(&v).unwrap();
        assert!(EnvVarValue::from_str(s).is_err());
    }
}
