use std::{
    cmp::{
        Ord,
        Ordering,
        PartialOrd,
    },
    fmt::{
        self,
        Debug,
        Display,
    },
    ops::Deref,
    str::FromStr,
};

use anyhow::Context;
use errors::ErrorMetadata;
use serde_json::Value as JsonValue;

use crate::{
    base64::{
        decode_urlsafe,
        encode_urlsafe,
    },
    heap_size::HeapSize,
    sha256::Sha256,
    table_name::TableIdentifier,
    TableNumber,
    TabletId,
    TabletIdAndTableNumber,
};

pub type ResolvedDocumentId = GenericDocumentId<TabletIdAndTableNumber>;
pub type InternalDocumentId = GenericDocumentId<TabletId>;
pub type DeveloperDocumentId = GenericDocumentId<TableNumber>;

/// A raw reference to a document. `DocumentId`s can appear in `Value`s as
/// `Id`s, `StrongRef`s, or `WeakRef`s.
#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Copy)]
pub struct GenericDocumentId<T: TableIdentifier> {
    table: T,
    internal_id: InternalId,
}

#[cfg(any(test, feature = "testing"))]
impl<T: TableIdentifier + proptest::arbitrary::Arbitrary> proptest::arbitrary::Arbitrary
    for GenericDocumentId<T>
{
    type Parameters = ();

    type Strategy = impl proptest::strategy::Strategy<Value = GenericDocumentId<T>>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        (any::<T>(), any::<InternalId>()).prop_map(|(t, id)| Self::new(t, id))
    }
}

impl<T: TableIdentifier> GenericDocumentId<T> {
    /// Create a new [`DocumentId`] from a [`TableName`] and an [`InternalId`].
    pub fn new(table: T, internal_id: InternalId) -> Self {
        Self { table, internal_id }
    }

    /// The table that the reference points into.
    pub fn table(&self) -> &T {
        &self.table
    }

    /// The ID of the document the reference points at.
    pub fn internal_id(&self) -> InternalId {
        self.internal_id
    }

    /// Minimum valid [`DocumentId`].
    pub fn min() -> Self {
        Self::new(<T as TableIdentifier>::min(), InternalId::MIN)
    }

    /// How large is the given `DocumentId`?
    pub fn size(&self) -> usize {
        self.table.size() + self.internal_id.size()
    }

    pub fn map_table<U: TableIdentifier>(
        self,
        f: impl Fn(T) -> anyhow::Result<U>,
    ) -> anyhow::Result<GenericDocumentId<U>> {
        Ok(GenericDocumentId::new(f(self.table)?, self.internal_id))
    }

    pub fn into_table_and_id(self) -> (T, InternalId) {
        (self.table, self.internal_id)
    }
}

impl<T: TableIdentifier> Debug for GenericDocumentId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl<T: TableIdentifier> From<GenericDocumentId<T>> for String {
    fn from(id: GenericDocumentId<T>) -> Self {
        (&id).into()
    }
}

impl<'a, T: TableIdentifier> From<&'a GenericDocumentId<T>> for String {
    fn from(id: &'a GenericDocumentId<T>) -> Self {
        id.table().document_id_to_string(id.internal_id)
    }
}

impl<T: TableIdentifier> From<GenericDocumentId<T>> for JsonValue {
    fn from(id: GenericDocumentId<T>) -> JsonValue {
        serde_json::Value::String(id.into())
    }
}

impl<T: TableIdentifier + FromStr<Err = anyhow::Error>> TryFrom<JsonValue>
    for GenericDocumentId<T>
{
    type Error = anyhow::Error;

    fn try_from(v: JsonValue) -> anyhow::Result<GenericDocumentId<T>> {
        if let JsonValue::String(s) = v {
            s.parse()
        } else {
            Err(anyhow::anyhow!("DocumentId must be a string JSON value."))
        }
    }
}

impl<T: TableIdentifier> Display for GenericDocumentId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", String::from(self.clone()))
    }
}

impl<T: TableIdentifier + FromStr<Err = anyhow::Error>> FromStr for GenericDocumentId<T> {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let id: Result<Self, anyhow::Error> = try {
            // Versions 4/5: DocumentIds have the table name and internal ID separated by
            // "|". Version 2 are 3 IDs are base64 encoded with an alphabet that
            // doesn't include "|".
            let (table_s, id_s) = s.split_once('|').context("Document IDs must contain '|'")?;
            Self {
                table: table_s.parse()?,
                internal_id: InternalId::from_str(id_s)?,
            }
        };
        id.context(ErrorMetadata::bad_request(
            "InvalidId",
            format!("Id `{s}` was invalid."),
        ))
    }
}

impl<T: TableIdentifier> HeapSize for GenericDocumentId<T> {
    fn heap_size(&self) -> usize {
        self.table.heap_size()
    }
}

impl DeveloperDocumentId {
    /// A `to_string` method for user-facing error messages.
    pub fn to_string_pretty(&self) -> String {
        format!("Id(\"{}\", \"{}\")", self.table, self.internal_id)
    }
}

impl From<ResolvedDocumentId> for InternalDocumentId {
    fn from(value: ResolvedDocumentId) -> Self {
        GenericDocumentId::new(value.table.tablet_id, value.internal_id)
    }
}

/// An internal ID serialized to 16 bytes.
///
/// 14 bytes of randomness followed by the day encoded in 2 bytes.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct InternalId(pub [u8; 16]);

impl InternalId {
    pub fn size(&self) -> usize {
        16
    }
}

impl PartialOrd for InternalId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for InternalId {
    fn cmp(&self, other: &Self) -> Ordering {
        self[..].cmp(&other[..])
    }
}

impl InternalId {
    /// A byte array that sorts after the bytes of every InternalId.
    pub const AFTER_ALL_BYTES: &'static [u8] = &[u8::MAX; 18];
    /// A byte array that sorts before the bytes of every InternalId.
    pub const BEFORE_ALL_BYTES: &'static [u8] = &[];
    /// The maximum valid InternalId in sorted order.
    pub const MAX: InternalId = Self([u8::MAX; 16]);
    pub const MAX_SIZE: usize = 17;
    /// The minimum valid InternalId in sorted order.
    pub const MIN: InternalId = Self([u8::MIN; 16]);

    pub fn from_developer_str(s: &str) -> anyhow::Result<Self> {
        // Developers apps may still provide V4 IDs, which are 17 bytes base 62 encoded,
        // or V5 IDs, which are 16 bytes base 64 encoded.

        // All IDs have been migrated to use the latter, so we parse both but only
        // return the V5 version.

        // The V4 encoder uses the `base_62` crate to encode 17 bytes with the change of
        // base algorithm. Note that log(256)/log(62) * 17 is 22.84, so the
        // output must be at least 23 bytes.
        let result = if s.len() > 22 {
            let vec = base_62::decode(s).map_err(|e| anyhow::anyhow!("Failed decode ID {e}"))?;
            let v4_bytes: [u8; 17] = vec[..].try_into()?;
            let randomness: [u8; 15] = v4_bytes[..15]
                .try_into()
                .expect("Could not read bytes from InternalId");
            let day: [u8; 2] = v4_bytes[15..17]
                .try_into()
                .expect("Could not read bytes from InternalId");
            let truncated: [u8; 14] = Sha256::hash(&randomness)[..14]
                .try_into()
                .expect("Could not truncate V4 ID");
            let mut reconstructed = [0u8; 16];
            reconstructed[..14].copy_from_slice(&truncated);
            reconstructed[14..16].copy_from_slice(&day);
            InternalId(reconstructed)
        }
        // Similarly, log(256)/log(64) * 16 is 21.33, so `base64` will output 22 bytes.
        else {
            let vec = decode_urlsafe(s)?;
            InternalId(vec[..].try_into()?)
        };
        Ok(result)
    }
}

impl Display for InternalId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", String::from(*self))
    }
}

impl Debug for InternalId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "InternalId({})", self)
    }
}

impl Deref for InternalId {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.0
    }
}

impl FromStr for InternalId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // All IDs in our system should be V5 IDs, which are 16 bytes base 64 encoded.
        // If parsing a developer provided ID, use `from_developer_str` which handles
        // the older V4 format.
        if s.len() > 22 {
            anyhow::bail!("Found V4 Internal ID");
        }
        let vec = decode_urlsafe(s)?;
        Ok(InternalId(vec[..].try_into()?))
    }
}

impl From<InternalId> for String {
    fn from(id: InternalId) -> Self {
        encode_urlsafe(&id.0[..])
    }
}

impl From<[u8; 16]> for InternalId {
    fn from(id: [u8; 16]) -> Self {
        InternalId(id)
    }
}

impl From<InternalId> for Vec<u8> {
    fn from(id: InternalId) -> Vec<u8> {
        id.0.into()
    }
}

impl<'a> TryFrom<&'a [u8]> for InternalId {
    type Error = anyhow::Error;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if let Ok(b) = <[u8; 16]>::try_from(value) {
            return Ok(InternalId::from(b));
        }
        anyhow::bail!(
            "Invalid InternalId length: Expected 16 bytes but received {}",
            value.len()
        );
    }
}

impl TryFrom<Vec<u8>> for InternalId {
    type Error = anyhow::Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(&value[..])
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::{
        InternalDocumentId,
        InternalId,
    };

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_document_id_roundtrip(v in any::<InternalDocumentId>()) {
            let roundtripped = String::from(v).parse().unwrap();
            assert_eq!(v, roundtripped);
        }

        #[test]
        fn test_internal_ids_within_byte_bounds(id in any::<InternalId>()) {
            let bytes = &id[..];
            assert!(InternalId::BEFORE_ALL_BYTES < bytes);
            assert!(InternalId::AFTER_ALL_BYTES > bytes);
        }
    }
}
