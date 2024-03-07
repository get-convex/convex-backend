use std::str::FromStr;

use slugify::slugify;
use value::Namespace;

pub const IDENTIFIER_PREFIX: &str = "source_";

/// An identifier that is not in the system namespace, possibly modified.
#[derive(Clone, Debug)]
pub struct ValidIdentifier<T: FromStr + Namespace>(pub T);

impl<T: FromStr + Namespace> FromStr for ValidIdentifier<T> {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        if let Ok(identifier) = s.parse::<T>()
            && !identifier.is_system()
        {
            Ok(Self(identifier))
        } else {
            let slugified = slugify!(s, separator = "_");
            let valid_str = prefix_field(&slugified);
            let identifier = valid_str
                .parse::<T>()
                .map_err(|_| anyhow::anyhow!("Failed to create a valid identifier."))?;
            Ok(Self(identifier))
        }
    }
}

pub fn prefix_field(s: &str) -> String {
    format!("{IDENTIFIER_PREFIX}{s}")
}
