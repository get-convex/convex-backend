//! Values that may contain commit timestamps that are not yet known.
//!
//! A transaction's commit timestamp is assigned by the committer after user
//! code has finished running, but code inside the transaction can refer to it
//! with the PendingValue::CommitTs variant.

use std::collections::BTreeMap;

use serde_json::{
    json,
    Value as JsonValue,
};

use crate::{
    array::check_array_len,
    object::check_field_count,
    size::{
        check_nesting,
        check_system_size,
        Size,
    },
    ConvexObject,
    ConvexValue,
    FieldName,
};

/// Wire token for a commit timestamp that is not yet known:
/// `{"$commitTs": null}`.
pub const COMMIT_TS_FIELD: &str = "$commitTs";

/// A value that may contain commit timestamps that are not yet known.
///
/// Invariant: `Object` and `Array` nodes appear only on paths leading to at
/// least one `CommitTs` leaf; a value with no unresolved commit timestamps is
/// always `Concrete`. The [`PendingValue::object`] and [`PendingValue::array`]
/// constructors maintain this, so structural equality is value equality.
///
/// This has a nice property where in the common case we see a Concrete value
/// and don't have to do any recursive computation to search for a CommitTs to
/// replace in the committer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PendingValue {
    /// A subtree containing no unresolved commit timestamps.
    Concrete(ConvexValue),
    /// The commit timestamp of the current transaction, unknown until the
    /// transaction commits. Resolves to an `Int64`.
    CommitTs,
    /// `size` and `nesting` are precomputed like [`crate::ConvexObject`]'s so
    /// that building nested values doesn't recompute them at every level.
    Object {
        fields: BTreeMap<FieldName, PendingValue>,
        size: usize,
        nesting: usize,
    },
    Array {
        values: Vec<PendingValue>,
        size: usize,
        nesting: usize,
    },
}

impl From<ConvexValue> for PendingValue {
    fn from(value: ConvexValue) -> Self {
        Self::Concrete(value)
    }
}

impl From<ConvexObject> for PendingValue {
    fn from(value: ConvexObject) -> Self {
        Self::Concrete(ConvexValue::Object(value))
    }
}

impl PendingValue {
    /// Does this value contain an unresolved commit timestamp?
    pub fn is_pending(&self) -> bool {
        match self {
            Self::Concrete(_) => false,
            Self::CommitTs | Self::Object { .. } | Self::Array { .. } => true,
        }
    }

    pub fn is_object(&self) -> bool {
        match self {
            PendingValue::Concrete(ConvexValue::Object(_)) | PendingValue::Object { .. } => true,
            PendingValue::Array { .. } | PendingValue::CommitTs | PendingValue::Concrete(_) => {
                false
            },
        }
    }

    /// Parse internal JSON, additionally accepting the `{"$commitTs": null}`
    /// token. Call this only at boundaries that may accept unresolved commit
    /// timestamps (document writes within a mutation, and eventually
    /// sub-function arguments and return values); everywhere else, the
    /// ordinary `JsonValue -> ConvexValue` conversion rejects the token.
    pub fn from_uncommitted_json(json: JsonValue) -> anyhow::Result<Self> {
        let value = match json {
            JsonValue::Object(map) => {
                if map.len() == 1
                    && let Some((key, token_value)) = map.iter().next()
                    && key.as_str() == COMMIT_TS_FIELD
                {
                    anyhow::ensure!(
                        token_value.is_null(),
                        "{COMMIT_TS_FIELD} value must be null"
                    );
                    return Ok(Self::CommitTs);
                }
                // Single-key `$` objects are opaque `ConvexValue` encodings
                // (e.g. `$integer`); delegating them whole also rejects any
                // token nested inside one.
                if map.len() == 1
                    && let Some(key) = map.keys().next()
                    && key.starts_with('$')
                {
                    return Ok(Self::Concrete(ConvexValue::try_from(JsonValue::Object(
                        map,
                    ))?));
                }
                let mut fields = BTreeMap::new();
                for (key, value) in map {
                    fields.insert(
                        key.parse::<FieldName>()?,
                        Self::from_uncommitted_json(value)?,
                    );
                }
                Self::object(fields)?
            },
            JsonValue::Array(items) => {
                let values = items
                    .into_iter()
                    .map(Self::from_uncommitted_json)
                    .collect::<anyhow::Result<Vec<_>>>()?;
                Self::array(values)?
            },
            leaf => Self::Concrete(ConvexValue::try_from(leaf)?),
        };
        Ok(value)
    }

    /// Build an object node, collapsing to `Concrete` when no field contains
    /// an unresolved commit timestamp.
    pub fn object(fields: BTreeMap<FieldName, PendingValue>) -> anyhow::Result<Self> {
        if fields.values().any(Self::is_pending) {
            check_field_count(fields.len())?;
            let size = 1
                + fields
                    .iter()
                    .map(|(name, value)| name.len() + 1 + value.size())
                    .sum::<usize>()
                + 1;
            let nesting = 1 + fields.values().map(Self::nesting).max().unwrap_or(0);
            check_system_size(size)?;
            check_nesting(nesting)?;
            return Ok(Self::Object {
                fields,
                size,
                nesting,
            });
        }
        let concrete: BTreeMap<FieldName, ConvexValue> = fields
            .into_iter()
            .map(|(name, value)| {
                let Self::Concrete(value) = value else {
                    unreachable!("non-pending value must be Concrete");
                };
                (name, value)
            })
            .collect();
        Ok(Self::Concrete(ConvexValue::Object(concrete.try_into()?)))
    }

    /// Build an array node, collapsing to `Concrete` when no element contains
    /// an unresolved commit timestamp.
    pub fn array(values: Vec<PendingValue>) -> anyhow::Result<Self> {
        if values.iter().any(Self::is_pending) {
            check_array_len(values.len())?;
            let size = 1 + values.iter().map(Self::size).sum::<usize>() + 1;
            let nesting = 1 + values.iter().map(Self::nesting).max().unwrap_or(0);
            check_system_size(size)?;
            check_nesting(nesting)?;
            return Ok(Self::Array {
                values,
                size,
                nesting,
            });
        }
        let concrete: Vec<ConvexValue> = values
            .into_iter()
            .map(|value| {
                let Self::Concrete(value) = value else {
                    unreachable!("non-pending value must be Concrete");
                };
                value
            })
            .collect();
        Ok(Self::Concrete(ConvexValue::Array(concrete.try_into()?)))
    }

    /// Encode to internal JSON, with `{"$commitTs": null}` at each unresolved
    /// commit timestamp. Inverse of [`PendingValue::from_uncommitted_json`].
    pub fn to_uncommitted_json(&self) -> JsonValue {
        match self {
            Self::Concrete(value) => value.to_internal_json(),
            Self::CommitTs => json!({ COMMIT_TS_FIELD: null }),
            Self::Object { fields, .. } => JsonValue::Object(
                fields
                    .iter()
                    .map(|(name, value)| (name.to_string(), value.to_uncommitted_json()))
                    .collect(),
            ),
            Self::Array { values, .. } => {
                JsonValue::Array(values.iter().map(Self::to_uncommitted_json).collect())
            },
        }
    }

    /// Replace each unresolved commit timestamp with `Int64(commit_ts)`.
    pub fn resolve(&self, commit_ts: i64) -> anyhow::Result<ConvexValue> {
        let value = match self {
            Self::Concrete(value) => value.clone(),
            Self::CommitTs => ConvexValue::Int64(commit_ts),
            Self::Object { fields, .. } => {
                let fields: BTreeMap<FieldName, ConvexValue> = fields
                    .iter()
                    .map(|(name, value)| anyhow::Ok((name.clone(), value.resolve(commit_ts)?)))
                    .collect::<anyhow::Result<_>>()?;
                ConvexValue::Object(fields.try_into()?)
            },
            Self::Array { values, .. } => {
                let values = values
                    .iter()
                    .map(|value| value.resolve(commit_ts))
                    .collect::<anyhow::Result<Vec<_>>>()?;
                ConvexValue::Array(values.try_into()?)
            },
        };
        Ok(value)
    }

    /// Decompose an object into its fields, e.g. to merge a patch into it.
    /// Fails if the value is not an object.
    pub fn into_object_fields(self) -> anyhow::Result<BTreeMap<FieldName, PendingValue>> {
        match self {
            Self::Concrete(ConvexValue::Object(object)) => Ok(BTreeMap::from(object)
                .into_iter()
                .map(|(name, value)| (name, Self::Concrete(value)))
                .collect()),
            Self::Object { fields, .. } => Ok(fields),
            Self::Concrete(_) | Self::CommitTs | Self::Array { .. } => {
                anyhow::bail!("Value must be an Object")
            },
        }
    }

    /// Extract the value if it contains no unresolved commit timestamps.
    pub fn try_into_concrete(self) -> anyhow::Result<ConvexValue> {
        match self {
            Self::Concrete(value) => Ok(value),
            _ => anyhow::bail!("Value contains an unresolved commit timestamp"),
        }
    }
}

/// Sizes and nesting match the resolved value exactly: an unresolved commit
/// timestamp resolves to an `Int64` of fixed width, so limits enforced before
/// resolution hold after it.
impl Size for PendingValue {
    fn size(&self) -> usize {
        match self {
            Self::Concrete(value) => value.size(),
            Self::CommitTs => 1 + 8,
            Self::Object { size, .. } | Self::Array { size, .. } => *size,
        }
    }

    fn nesting(&self) -> usize {
        match self {
            Self::Concrete(value) => value.nesting(),
            Self::CommitTs => 0,
            Self::Object { nesting, .. } | Self::Array { nesting, .. } => *nesting,
        }
    }
}
