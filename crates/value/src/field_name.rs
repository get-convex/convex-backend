use std::{
    borrow::Borrow,
    fmt::{
        self,
        Debug,
    },
    ops::Deref,
    str::FromStr,
};

use compact_str::CompactString;
use sync_types::identifier::{
    check_valid_field_name,
    check_valid_identifier,
};

use crate::{
    heap_size::HeapSize,
    ConvexValue,
    Namespace,
};

/// Field names within an object type.
#[derive(Hash, Eq, Ord, PartialEq, PartialOrd, Clone, derive_more::Display)]
pub struct FieldName(CompactString);

impl Namespace for FieldName {
    fn is_system(&self) -> bool {
        self.0.starts_with('_')
    }
}

impl FromStr for FieldName {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        check_valid_field_name(s)?;
        Ok(Self(s.into()))
    }
}

impl TryFrom<String> for FieldName {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        check_valid_field_name(&s)?;
        Ok(Self(s.into()))
    }
}

impl From<FieldName> for String {
    fn from(f: FieldName) -> Self {
        f.0.into()
    }
}

impl Debug for FieldName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Deref for FieldName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Borrow<str> for FieldName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl HeapSize for FieldName {
    fn heap_size(&self) -> usize {
        self.0.heap_size()
    }
}

impl From<FieldName> for ConvexValue {
    fn from(value: FieldName) -> Self {
        ConvexValue::String(
            String::from(value.0)
                .try_into()
                .expect("Field name was unexpectedly not a valid Convex string"),
        )
    }
}

#[derive(Default, Clone, Copy)]
pub enum FieldType {
    #[default]
    Either,
    User,
    System,
    UserIdentifier,
}

/// Field names within an object that are also valid identifiers.
#[derive(Hash, Eq, Ord, PartialEq, PartialOrd, Clone, Debug, derive_more::Display)]
pub struct IdentifierFieldName(CompactString);

impl HeapSize for IdentifierFieldName {
    fn heap_size(&self) -> usize {
        self.0.heap_size()
    }
}

impl Namespace for IdentifierFieldName {
    fn is_system(&self) -> bool {
        self.0.starts_with('_')
    }
}

impl FromStr for IdentifierFieldName {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        check_valid_field_name(s)?;
        check_valid_identifier(s)?;
        Ok(Self(s.into()))
    }
}

impl From<IdentifierFieldName> for String {
    fn from(f: IdentifierFieldName) -> Self {
        f.0.into()
    }
}

impl Deref for IdentifierFieldName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Borrow<str> for IdentifierFieldName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl From<IdentifierFieldName> for FieldName {
    fn from(value: IdentifierFieldName) -> Self {
        // All identifier field names are also field names
        FieldName(value.0)
    }
}

impl TryFrom<FieldName> for IdentifierFieldName {
    type Error = anyhow::Error;

    fn try_from(value: FieldName) -> Result<Self, Self::Error> {
        check_valid_identifier(&value)?;
        Ok(IdentifierFieldName(value.0))
    }
}
