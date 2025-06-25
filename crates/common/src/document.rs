//! Documents and document identifiers.
//!
//! This is the authoritative representation of a document within the database.
use std::{
    cmp::Ordering,
    collections::BTreeMap,
    fmt::{
        self,
        Debug,
        Display,
    },
    num::FpCategory,
    ops::{
        Deref,
        DerefMut,
    },
    sync::LazyLock,
    time::Duration,
};

use anyhow::Context;
use errors::ErrorMetadata;
use float_next_after::NextAfter;
use itertools::Itertools;
use packed_value::{
    ByteBuffer,
    OpenedValue,
    PackedValue,
};
use pb::common::{
    DocumentUpdate as DocumentUpdateProto,
    DocumentUpdateWithPrevTs as DocumentUpdateWithPrevTsProto,
    ResolvedDocument as ResolvedDocumentProto,
};
#[cfg(any(test, feature = "testing"))]
use proptest::prelude::*;
use serde_json::{
    Number,
    Value as JsonValue,
};
pub use value::InternalId;
use value::{
    check_nesting_for_documents,
    export::ValueFormat,
    heap_size::HeapSize,
    id_v6::DeveloperDocumentId,
    serde::ConvexSerializable,
    sorting::{
        write_sort_key,
        write_sort_key_or_undefined,
    },
    walk::ConvexValueType,
    ConvexObject,
    ConvexValue,
    FieldName,
    FieldPath,
    IdentifierFieldName,
    InternalDocumentId,
    Namespace,
    ResolvedDocumentId,
    TableNumber,
    TabletId,
    MAX_DOCUMENT_NESTING,
};

#[cfg(any(test, feature = "testing"))]
use crate::value::FieldType;
use crate::{
    floating_point::MAX_EXACT_F64_INT,
    index::{
        IndexKey,
        IndexKeyBytes,
    },
    pii::PII,
    types::{
        PersistenceVersion,
        Timestamp,
    },
    value::Size,
};

/// The database automatically inserts the assigned document ID as an "_id"
/// field.
pub static ID_FIELD: LazyLock<IdentifierFieldName> = LazyLock::new(|| "_id".parse().unwrap());

pub static ID_FIELD_PATH: LazyLock<FieldPath> =
    LazyLock::new(|| FieldPath::new(vec![ID_FIELD.clone()]).unwrap());

/// The database automatically inserts the creation time in each document with
/// the "_creationTime" field. The timestamp is a Float64 of milliseconds since
/// the Unix epoch.
pub static CREATION_TIME_FIELD: LazyLock<IdentifierFieldName> =
    LazyLock::new(|| "_creationTime".parse().unwrap());

pub static CREATION_TIME_FIELD_PATH: LazyLock<FieldPath> =
    LazyLock::new(|| FieldPath::new(vec![CREATION_TIME_FIELD.clone()]).unwrap());

// The current Unix timestamp (as of 2022-08-02) in milliseconds is
//
//     1659481438151.257
//
// which is represented as a 64-bit floating point number as
//
//     0    10000100111 1000001001100000110011111100000000010001100001101111
//     sign exponent    mantissa
//
// The distance to the next floating point number is then
//
//     2^(0b10000100111 - 1023) * 2^-52
//     = 0.000244140625 ms
//     = 244.140625 ns
//
// This precision holds until the mantissa is entirely ones:
//
//     0 10000100111 1111111111111111111111111111111111111111111111111111
//     = 2199023255551.9998 ms
//     ~ 17 years in the future
//
// If we increment the exponent by one, we get ~86 years in the future with
// a precision of 488.28125 ns. We'll just go with that to be conservative.
pub const CREATION_TIME_PRECISION: Duration = Duration::from_nanos(500);

#[derive(Clone, Copy)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct CreationTime {
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "(1.)..33224882812.")
    )]
    ts_ms: f64,
}

impl Ord for CreationTime {
    fn cmp(&self, other: &Self) -> Ordering {
        self.ts_ms.total_cmp(&other.ts_ms)
    }
}

impl PartialOrd for CreationTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for CreationTime {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for CreationTime {}

impl TryFrom<f64> for CreationTime {
    type Error = anyhow::Error;

    fn try_from(ts_ms: f64) -> anyhow::Result<Self> {
        match ts_ms.classify() {
            FpCategory::Zero | FpCategory::Normal | FpCategory::Subnormal => {
                anyhow::ensure!(!ts_ms.is_sign_negative())
            },
            FpCategory::Infinite | FpCategory::Nan => {
                anyhow::bail!("Invalid creation time {ts_ms}")
            },
        }
        Ok(CreationTime { ts_ms })
    }
}

impl TryFrom<Timestamp> for CreationTime {
    type Error = anyhow::Error;

    fn try_from(ts: Timestamp) -> anyhow::Result<Self> {
        Self::try_from(timestamp_to_ms(ts)?)
    }
}

impl From<CreationTime> for f64 {
    fn from(t: CreationTime) -> f64 {
        t.ts_ms
    }
}

pub fn timestamp_to_ms(ts: Timestamp) -> anyhow::Result<f64> {
    let nanos = u64::from(ts);

    // 1e6 nanoseconds in a millisecond.
    let ms_integral = nanos / 1_000_000;
    anyhow::ensure!(ms_integral <= (MAX_EXACT_F64_INT as u64));

    let ms_fractional = ((nanos % 1_000_000) as f64) * 1e-6;
    Ok(ms_integral as f64 + ms_fractional)
}

impl TryFrom<CreationTime> for JsonValue {
    type Error = anyhow::Error;

    fn try_from(t: CreationTime) -> anyhow::Result<JsonValue> {
        let n = Number::from_f64(Into::<f64>::into(t))
            .ok_or_else(|| anyhow::anyhow!("f64 failed to convert to json number."))?;
        Ok(JsonValue::Number(n))
    }
}

impl fmt::Debug for CreationTime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.ts_ms)
    }
}

impl CreationTime {
    // CreationTime::ONE is a default for tests. We don't use zero because zero
    // is a likely value that a bug may produce in prod, so it is invalid.
    #[cfg(any(test, feature = "testing"))]
    pub const ONE: Self = Self { ts_ms: 1. };

    pub fn increment(&mut self) -> anyhow::Result<Self> {
        let result = *self;

        let next_float = self.ts_ms.next_after(f64::INFINITY);
        *self = Self::try_from(next_float)?;

        Ok(result)
    }
}

/// Documents store [`Value`]s.
/// DeveloperDocument is the public-facing document type.
#[derive(Clone, Eq, PartialEq)]
pub struct DeveloperDocument {
    id: DeveloperDocumentId,
    creation_time: CreationTime,
    value: PII<ConvexObject>,
}

impl DeveloperDocument {
    pub fn creation_time(&self) -> CreationTime {
        self.creation_time
    }

    /// The [`InternalId`] associated with the document's ID.
    pub fn internal_id(&self) -> InternalId {
        self.id.internal_id()
    }

    /// The body/payload of the document
    pub fn value(&self) -> &PII<ConvexObject> {
        &self.value
    }

    /// Consume the document's value.
    pub fn into_value(self) -> PII<ConvexObject> {
        self.value
    }

    pub fn size(&self) -> usize {
        self.id.size() + self.value.size()
    }

    pub fn to_internal_json(&self) -> JsonValue {
        self.value.0.to_internal_json()
    }
}

impl HeapSize for DeveloperDocument {
    fn heap_size(&self) -> usize {
        self.id.heap_size() + self.value.heap_size()
    }
}

impl Debug for DeveloperDocument {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Document({:?})", self.value)
    }
}

impl Display for DeveloperDocument {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Document(value: {})", self.value.0)
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct ResolvedDocument {
    tablet_id: TabletId,
    document: DeveloperDocument,
}

impl Deref for ResolvedDocument {
    type Target = DeveloperDocument;

    fn deref(&self) -> &Self::Target {
        &self.document
    }
}

impl Debug for ResolvedDocument {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(&self.document, f)
    }
}

impl Display for ResolvedDocument {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.document, f)
    }
}

impl HeapSize for ResolvedDocument {
    fn heap_size(&self) -> usize {
        self.document.heap_size()
    }
}

impl TryFrom<ResolvedDocument> for ResolvedDocumentProto {
    type Error = anyhow::Error;

    fn try_from(
        ResolvedDocument {
            tablet_id,
            document:
                DeveloperDocument {
                    id,
                    creation_time,
                    value,
                },
        }: ResolvedDocument,
    ) -> anyhow::Result<Self> {
        let value = value.0.json_serialize()?.into_bytes();
        let id = ResolvedDocumentId {
            tablet_id,
            developer_id: id,
        };
        Ok(Self {
            id: Some(id.into()),
            creation_time: Some(creation_time.into()),
            value: Some(value),
        })
    }
}

impl TryFrom<ResolvedDocumentProto> for ResolvedDocument {
    type Error = anyhow::Error;

    fn try_from(
        ResolvedDocumentProto {
            id,
            creation_time,
            value,
        }: ResolvedDocumentProto,
    ) -> anyhow::Result<Self> {
        let id: ResolvedDocumentId = id
            .ok_or_else(|| anyhow::anyhow!("Missing id"))?
            .try_into()?;
        let creation_time = creation_time
            .ok_or_else(|| anyhow::anyhow!("Missing creation time"))?
            .try_into()?;
        let value: ConvexObject = serde_json::from_slice::<JsonValue>(
            &value.ok_or_else(|| anyhow::anyhow!("Missing value"))?,
        )?
        .try_into()?;

        Ok(Self {
            tablet_id: id.tablet_id,
            document: DeveloperDocument {
                id: id.into(),
                creation_time,
                value: PII(value),
            },
        })
    }
}

impl ResolvedDocument {
    pub fn new(
        id: ResolvedDocumentId,
        creation_time: CreationTime,
        mut value: ConvexObject,
    ) -> anyhow::Result<Self> {
        let id_value: ConvexValue = id.into();
        if let Some(existing_value) = value.get(&FieldName::from(ID_FIELD.clone())) {
            if existing_value != &id_value {
                anyhow::bail!(ErrorMetadata::bad_request(
                    "InvalidIdError",
                    format!(
                        "Provided document ID {id_value} doesn't match '_id' field \
                         {existing_value}"
                    ),
                ));
            }
        } else {
            let mut fields: BTreeMap<_, _> = value.into();
            fields.insert(ID_FIELD.to_owned().into(), id_value);
            value = fields.try_into()?;
        }
        let creation_time_value = ConvexValue::from(f64::from(creation_time));
        match (
            creation_time_value,
            value.get(&FieldName::from(CREATION_TIME_FIELD.clone())),
        ) {
            (time, None) => {
                let mut fields: BTreeMap<_, _> = value.into();
                fields.insert(CREATION_TIME_FIELD.to_owned().into(), time);
                value = fields.try_into()?;
            },
            (l, r) if Some(&l) == r => (),
            _ => anyhow::bail!(ErrorMetadata::bad_request(
                "InvalidCreationTimeError",
                format!(
                    "Provided creation time {creation_time:?} doesn't match '_creationTime' field \
                     in {value}"
                ),
            )),
        }
        let doc = Self {
            tablet_id: id.tablet_id,
            document: DeveloperDocument {
                id: id.into(),
                creation_time,
                value: PII(value),
            },
        };
        doc.must_validate()?;
        Ok(doc)
    }

    fn must_validate(&self) -> anyhow::Result<()> {
        let violations = self.validate();
        if !violations.is_empty() {
            let msg = format!(
                "{self} isn't a valid document: {}",
                violations.into_iter().map(|v| v.to_string()).join("\n ")
            );
            anyhow::bail!(ErrorMetadata::bad_request("InvalidDocumentError", msg));
        }
        Ok(())
    }

    /// Checks system fields _id and _creationTime and checks that there aren't
    /// any other top-level system fields.
    /// Returns vec of violations, which may be displayed to clients.
    pub fn validate(&self) -> Vec<DocumentValidationError> {
        let mut violations = vec![];

        let nesting = self.value().nesting();
        if check_nesting_for_documents(nesting) {
            violations.push(DocumentValidationError::TooNested(nesting));
        }

        match self.value.get(&FieldName::from(ID_FIELD.clone())) {
            Some(ConvexValue::String(s)) => {
                if let Ok(document_id) = DeveloperDocumentId::decode(s) {
                    if document_id.table() != self.id.table() {
                        violations.push(DocumentValidationError::IdWrongTable);
                    } else if document_id.internal_id() != self.internal_id() {
                        violations.push(DocumentValidationError::IdMismatch(
                            self.id,
                            ConvexValue::String(s.clone()),
                        ));
                    }
                }
            },
            Some(v) => violations.push(DocumentValidationError::IdBadType(v.clone())),
            None => violations.push(DocumentValidationError::IdMissing),
        }
        match self
            .value
            .get(&FieldName::from(CREATION_TIME_FIELD.clone()))
        {
            Some(creation_time_value) => {
                if let ConvexValue::Float64(creation_time) = creation_time_value {
                    // Intentionally exclude -0, -inf, inf, and NaN.
                    if !(*creation_time > 0.0 && *creation_time < MAX_EXACT_F64_INT as f64) {
                        violations.push(DocumentValidationError::CreationTimeInvalidFloat(
                            *creation_time,
                        ));
                    }
                } else {
                    violations.push(DocumentValidationError::CreationTimeBadType(
                        creation_time_value.clone(),
                    ));
                }
            },
            None => violations.push(DocumentValidationError::CreationTimeMissing),
        }
        for (field, _) in self.value.iter() {
            if field == &(*ID_FIELD).clone().into()
                || field == &(*CREATION_TIME_FIELD).clone().into()
            {
                continue;
            }
            if field.is_system() {
                violations.push(DocumentValidationError::SystemField(field.clone()));
            }
        }
        violations
    }

    /// Returns the set of values that this document should be indexed by for
    /// the given fields if they exist in the document
    pub fn index_key(
        &self,
        fields: &[FieldPath],
        _persistence_version: PersistenceVersion,
    ) -> IndexKey {
        let mut values = vec![];
        for field in fields.iter() {
            if let Some(v) = self.value.get_path(field) {
                values.push(Some(v.clone()));
            } else {
                values.push(None);
            }
        }
        IndexKey::new_allow_missing(values, self.developer_id())
    }

    /// Recreate a `Document` from an already-written value to the database.
    /// This method assumes that system-provided fields, like `_id`, have
    /// already been inserted into `value`.
    pub fn from_database(tablet_id: TabletId, value: ConvexValue) -> anyhow::Result<Self> {
        let object: ConvexObject = value.try_into()?;
        let id = match object.get(&FieldName::from(ID_FIELD.clone())) {
            Some(ConvexValue::String(s)) => DeveloperDocumentId::decode(s)?,
            _ => anyhow::bail!("Object {} missing _id field", object),
        };
        let creation_time = match object.get(&FieldName::from(CREATION_TIME_FIELD.clone())) {
            Some(ConvexValue::Float64(ts)) => (*ts).try_into()?,
            None => anyhow::bail!("Object {object} missing _creationTime field"),
            _ => anyhow::bail!("Object {object} has invalid _creationTime field"),
        };
        Ok(Self {
            tablet_id,
            document: DeveloperDocument {
                id,
                creation_time,
                value: PII(object),
            },
        })
    }

    pub fn from_packed(
        value: ConvexValue,
        document_id: ResolvedDocumentId,
    ) -> anyhow::Result<Self> {
        let object: ConvexObject = value.try_into()?;
        let creation_time = match object.get(&FieldName::from(CREATION_TIME_FIELD.clone())) {
            Some(ConvexValue::Float64(ts)) => (*ts).try_into()?,
            None => anyhow::bail!("Object {object} missing _creationTime field"),
            _ => anyhow::bail!("Object {object} has invalid _creationTime field"),
        };
        Ok(Self {
            tablet_id: document_id.tablet_id,
            document: DeveloperDocument {
                id: document_id.into(),
                creation_time,
                value: PII(object),
            },
        })
    }

    /// Return a new [`Document`] with the value updated to `new_value`. If
    /// `new_value` contains an `_id` field, it must match the current `_id`
    /// field's value.
    pub fn replace_value(&self, new_value: ConvexObject) -> anyhow::Result<Self> {
        Self::new(self.id(), self.creation_time, new_value)
    }

    pub fn into_value(self) -> PII<ConvexObject> {
        self.document.into_value()
    }

    pub fn to_developer(self) -> DeveloperDocument {
        self.document
    }

    pub fn id_with_table_id(&self) -> InternalDocumentId {
        InternalDocumentId::new(self.tablet_id, self.id.internal_id())
    }

    pub fn id(&self) -> ResolvedDocumentId {
        ResolvedDocumentId::new(self.tablet_id, self.id)
    }

    pub fn developer_id(&self) -> DeveloperDocumentId {
        self.id
    }

    pub fn export(self, format: ValueFormat) -> JsonValue {
        self.document.into_value().0.export(format)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct DocumentUpdateWithPrevTs {
    pub id: ResolvedDocumentId,
    /// The old document and its timestamp in the document log.
    /// The timestamp will become the update's `prev_ts`.
    pub old_document: Option<(ResolvedDocument, Timestamp)>,
    pub new_document: Option<ResolvedDocument>,
}

impl HeapSize for DocumentUpdateWithPrevTs {
    fn heap_size(&self) -> usize {
        self.old_document.heap_size() + self.new_document.heap_size()
    }
}

impl TryFrom<DocumentUpdateWithPrevTs> for DocumentUpdateWithPrevTsProto {
    type Error = anyhow::Error;

    fn try_from(
        DocumentUpdateWithPrevTs {
            id,
            old_document,
            new_document,
        }: DocumentUpdateWithPrevTs,
    ) -> anyhow::Result<Self> {
        let (old_document, old_ts) = old_document.unzip();
        Ok(Self {
            id: Some(id.into()),
            old_document: old_document.map(|d| d.try_into()).transpose()?,
            old_ts: old_ts.map(|ts| ts.into()),
            new_document: new_document.map(|d| d.try_into()).transpose()?,
        })
    }
}

impl TryFrom<DocumentUpdateWithPrevTsProto> for DocumentUpdateWithPrevTs {
    type Error = anyhow::Error;

    fn try_from(
        DocumentUpdateWithPrevTsProto {
            id,
            old_document,
            old_ts,
            new_document,
        }: DocumentUpdateWithPrevTsProto,
    ) -> anyhow::Result<Self> {
        let id = id
            .context("Document updates missing document id")?
            .try_into()?;
        Ok(Self {
            id,
            old_document: old_document
                .map(|d| {
                    anyhow::Ok((
                        d.try_into()?,
                        Timestamp::try_from(old_ts.context("old_ts missing")?)?,
                    ))
                })
                .transpose()?,
            new_document: new_document.map(|d| d.try_into()).transpose()?,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct DocumentUpdate {
    pub id: ResolvedDocumentId,
    pub old_document: Option<ResolvedDocument>,
    pub new_document: Option<ResolvedDocument>,
}

impl TryFrom<DocumentUpdate> for DocumentUpdateProto {
    type Error = anyhow::Error;

    fn try_from(
        DocumentUpdate {
            id,
            old_document,
            new_document,
        }: DocumentUpdate,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            id: Some(id.into()),
            old_document: old_document.map(|d| d.try_into()).transpose()?,
            new_document: new_document.map(|d| d.try_into()).transpose()?,
        })
    }
}

impl TryFrom<DocumentUpdateProto> for DocumentUpdate {
    type Error = anyhow::Error;

    fn try_from(
        DocumentUpdateProto {
            id,
            old_document,
            new_document,
        }: DocumentUpdateProto,
    ) -> anyhow::Result<Self> {
        let id = id
            .context("Document updates missing document id")?
            .try_into()?;
        Ok(Self {
            id,
            old_document: old_document.map(|d| d.try_into()).transpose()?,
            new_document: new_document.map(|d| d.try_into()).transpose()?,
        })
    }
}

impl From<DocumentUpdateWithPrevTs> for DocumentUpdate {
    fn from(update: DocumentUpdateWithPrevTs) -> Self {
        Self {
            id: update.id,
            old_document: update.old_document.map(|(d, _)| d),
            new_document: update.new_document,
        }
    }
}

/// Either a [`DocumentUpdate`] or a [`DocumentUpdateWithPrevTs`]
pub trait DocumentUpdateRef {
    fn id(&self) -> ResolvedDocumentId;
    fn old_document(&self) -> Option<&ResolvedDocument>;
    fn new_document(&self) -> Option<&ResolvedDocument>;
}

impl DocumentUpdateRef for DocumentUpdateWithPrevTs {
    fn id(&self) -> ResolvedDocumentId {
        self.id
    }

    fn old_document(&self) -> Option<&ResolvedDocument> {
        self.old_document.as_ref().map(|(d, _)| d)
    }

    fn new_document(&self) -> Option<&ResolvedDocument> {
        self.new_document.as_ref()
    }
}

impl DocumentUpdateRef for DocumentUpdate {
    fn id(&self) -> ResolvedDocumentId {
        self.id
    }

    fn old_document(&self) -> Option<&ResolvedDocument> {
        self.old_document.as_ref()
    }

    fn new_document(&self) -> Option<&ResolvedDocument> {
        self.new_document.as_ref()
    }
}

impl DeveloperDocument {
    pub fn new(id: DeveloperDocumentId, creation_time: CreationTime, value: ConvexObject) -> Self {
        Self {
            id,
            creation_time,
            value: PII(value),
        }
    }

    pub fn to_resolved(self, tablet_id: TabletId) -> ResolvedDocument {
        ResolvedDocument {
            tablet_id,
            document: self,
        }
    }

    pub fn id(&self) -> DeveloperDocumentId {
        self.id
    }

    pub fn table(&self) -> TableNumber {
        self.id.table()
    }
}

/// Two packed values, the actual document value and the document ID. The
/// document ID will contain more information that the `_id` field of the value
/// when we are using ID strings.
#[derive(Clone, Debug)]
#[cfg_attr(any(test, feature = "testing"), derive(PartialEq))]
pub struct PackedDocument(PackedValue<ByteBuffer>, ResolvedDocumentId);

impl PackedDocument {
    pub fn pack(document: &ResolvedDocument) -> Self {
        let document_id = document.id();
        let value = document.document.value();
        Self(PackedValue::pack_object(value), document_id)
    }

    pub fn unpack(&self) -> ResolvedDocument {
        let value = ConvexValue::try_from(self.0.as_ref()).expect("Couldn't unpack packed value");
        let document_id = self.1;
        ResolvedDocument::from_packed(value, document_id)
            .expect("Packed value wasn't a valid document?")
    }

    /// Same behavior as ResolvedDocument::id but you don't have to fully
    /// unpack.
    pub fn id(&self) -> ResolvedDocumentId {
        self.1
    }

    pub fn developer_id(&self) -> DeveloperDocumentId {
        self.id().developer_id
    }

    pub fn value(&self) -> &PackedValue<ByteBuffer> {
        &self.0
    }

    /// Like ResolvedDocument::index_key().into_bytes(), but you don't have to
    /// fully unpack.
    ///
    /// `buffer` is an existing allocation that will be cleared and reused.
    pub fn index_key<'a>(
        &self,
        fields: &[FieldPath],
        _persistence_version: PersistenceVersion,
        buffer: &'a mut IndexKeyBuffer,
    ) -> &'a IndexKeyBytes {
        let out = &mut buffer.0 .0;
        out.clear();
        for field_path in fields {
            let value = self.0.as_ref().open_path(field_path);
            write_sort_key_or_undefined(value, out).expect("failed to unpack opened value");
        }
        let Ok(()) = write_sort_key(
            self.id().developer_id.encode_into(&mut Default::default()),
            out,
        );
        &buffer.0
    }

    pub fn index_key_owned(
        &self,
        fields: &[FieldPath],
        persistence_version: PersistenceVersion,
    ) -> IndexKeyBytes {
        let mut buffer = IndexKeyBuffer::new();
        self.index_key(fields, persistence_version, &mut buffer);
        buffer.0
    }
}

/// A reusable allocation for use by `PackedDocument::index_key`
pub struct IndexKeyBuffer(IndexKeyBytes);
impl IndexKeyBuffer {
    pub fn new() -> Self {
        Self(IndexKeyBytes(Vec::new()))
    }
}

impl HeapSize for PackedDocument {
    fn heap_size(&self) -> usize {
        self.0.heap_size() + self.1.heap_size()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct ParsedDocument<D> {
    id: ResolvedDocumentId,
    creation_time: CreationTime,
    value: D,
}

impl<D> ParsedDocument<D> {
    pub fn id(&self) -> ResolvedDocumentId {
        self.id
    }

    pub fn developer_id(&self) -> DeveloperDocumentId {
        self.id.into()
    }

    pub fn creation_time(&self) -> CreationTime {
        self.creation_time
    }

    pub fn into_value(self) -> D {
        self.value
    }

    pub fn into_id_and_value(self) -> (ResolvedDocumentId, D) {
        (self.id, self.value)
    }

    pub fn map<E>(
        self,
        f: impl FnOnce(D) -> anyhow::Result<E>,
    ) -> anyhow::Result<ParsedDocument<E>> {
        Ok(ParsedDocument {
            id: self.id,
            creation_time: self.creation_time,
            value: f(self.value)?,
        })
    }
}

pub trait ParseDocument<D> {
    fn parse(self) -> anyhow::Result<ParsedDocument<D>>;
}

// this impl can't use ConvexSerializable because it's used with some types that
// directly impl `From<ConvexObject>`
impl<D> ParseDocument<D> for ResolvedDocument
where
    D: TryFrom<ConvexObject, Error = anyhow::Error>,
{
    fn parse(self) -> anyhow::Result<ParsedDocument<D>> {
        let id = self.id();
        let creation_time = self.creation_time;
        let value: D = self
            .document
            .into_value()
            .0
            .try_into()
            .with_context(|| format!("Failed to parse document id: {id}"))?;
        Ok(ParsedDocument {
            id,
            creation_time,
            value,
        })
    }
}

impl<D: ConvexSerializable> ParseDocument<D> for &ResolvedDocument {
    fn parse(self) -> anyhow::Result<ParsedDocument<D>> {
        let id = self.id();
        let creation_time = self.creation_time;
        let value: D = value::serde::from_value::<_, D::Serialized>(
            ConvexValueType::<&ConvexValue>::Object(&self.document.value().0),
        )?
        .try_into()
        .map_err(Into::<anyhow::Error>::into)
        .with_context(|| format!("Failed to parse document id: {id}"))?;
        Ok(ParsedDocument {
            id,
            creation_time,
            value,
        })
    }
}

impl<D: ConvexSerializable> ParseDocument<D> for &PackedDocument {
    fn parse(self) -> anyhow::Result<ParsedDocument<D>> {
        let creation_time = match self.value().as_ref().open()? {
            OpenedValue::Object(o) => match o.get(&CREATION_TIME_FIELD)? {
                Some(OpenedValue::Float64(ts)) => CreationTime::try_from(ts)?,
                None => anyhow::bail!("PackedDocument missing _creationTime field"),
                _ => anyhow::bail!("PackedDocument has non-float64 _creationTime field"),
            },
            v => anyhow::bail!("PackedDocument is {v:?}, not object"),
        };
        Ok(ParsedDocument {
            id: self.1,
            creation_time,
            value: self.0.as_ref().parse()?,
        })
    }
}

impl<D: ConvexSerializable> ParseDocument<D> for PackedDocument {
    fn parse(self) -> anyhow::Result<ParsedDocument<D>> {
        (&self).parse()
    }
}

impl<D> Deref for ParsedDocument<D> {
    type Target = D;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<D> DerefMut for ParsedDocument<D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DocumentValidationError {
    #[error(
        "Field '{0}' starts with an underscore, which is only allowed for system fields like '_id'"
    )]
    SystemField(FieldName),
    #[error("The document belongs to a different table than its '_id' field")]
    IdWrongTable,
    #[error("The document has id {0}, but its '_id' field is {1}")]
    IdMismatch(DeveloperDocumentId, ConvexValue),
    #[error("The '_id' field {0} must be an Id")]
    IdBadType(ConvexValue),
    #[error("The '_id' field is missing")]
    IdMissing,
    #[error("The '_creationTime' field should be a timestamp in milliseconds, is {0}")]
    CreationTimeInvalidFloat(f64),
    #[error("The '_creationTime' field should be a float, is {0}")]
    CreationTimeBadType(ConvexValue),
    #[error("The '_creationTime' field is missing")]
    CreationTimeMissing,
    #[error(
        "Document is too nested (nested {0} levels deep > maximum nesting {MAX_DOCUMENT_NESTING})"
    )]
    TooNested(usize),
}

impl DocumentValidationError {
    pub fn violation(&self) -> &'static str {
        match self {
            DocumentValidationError::SystemField(_) => "invalid system field",
            DocumentValidationError::IdWrongTable => "_id wrong table",
            DocumentValidationError::IdMismatch(..) => "_id mismatch",
            DocumentValidationError::IdBadType(_) => "_id is not an Id",
            DocumentValidationError::IdMissing => "_id missing",
            DocumentValidationError::CreationTimeInvalidFloat(_) => "_creationTime invalid float",
            DocumentValidationError::CreationTimeBadType(_) => "_creationTime wrong type",
            DocumentValidationError::CreationTimeMissing => "_creationTime missing",
            DocumentValidationError::TooNested(_) => "too nested",
        }
    }
}

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for ResolvedDocument {
    type Parameters = ();

    type Strategy = impl proptest::strategy::Strategy<Value = ResolvedDocument>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        use value::proptest::{
            ExcludeSetsAndMaps,
            RestrictNaNs,
            ValueBranching,
        };
        any_with::<(ResolvedDocumentId, CreationTime, ConvexObject)>((
            (),
            (),
            (
                prop::collection::SizeRange::default(),
                FieldType::User,
                ValueBranching::default(),
                ExcludeSetsAndMaps(false),
                RestrictNaNs(false),
            ),
        ))
        .prop_filter_map(
            "Invalid generated object for document",
            |(id, creation_time, object)| {
                let mut object = BTreeMap::from(object);
                object.insert(ID_FIELD.clone().into(), id.into());
                object.insert(
                    CREATION_TIME_FIELD.clone().into(),
                    ConvexValue::from(f64::from(creation_time)),
                );
                let value = ConvexObject::try_from(object).unwrap();
                let doc = ResolvedDocument::new(id, creation_time, value);
                doc.ok()
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use std::{
        assert_eq,
        collections::BTreeMap,
        str::FromStr as _,
    };

    use cmd_util::env::env_config;
    use proptest::prelude::*;
    use sync_types::testing::assert_roundtrips;
    use value::{
        id_v6::DeveloperDocumentId,
        proptest::{
            RestrictNaNs,
            ValueBranching,
        },
        ConvexObject,
        ConvexValue,
        ExcludeSetsAndMaps,
        FieldType,
        IdentifierFieldName,
        InternalId,
        ResolvedDocumentId,
        TableMapping,
        TableName,
        TableNamespace,
        TableNumber,
        TabletId,
    };

    use super::{
        CreationTime,
        DocumentUpdate,
        DocumentUpdateProto,
        DocumentUpdateWithPrevTs,
        DocumentUpdateWithPrevTsProto,
        IndexKeyBuffer,
        PackedDocument,
        ResolvedDocument,
        ResolvedDocumentProto,
    };
    use crate::{
        assert_obj,
        document::{
            CREATION_TIME_FIELD,
            ID_FIELD,
        },
        paths::FieldPath,
        types::PersistenceVersion,
    };
    #[test]
    fn test_map_table() -> anyhow::Result<()> {
        let internal_id = InternalId::MAX;
        let tablet_id = TabletId::MIN;
        let table_number = TableNumber::MIN;
        let table_name: TableName = "hewo".parse()?;
        let mut table_mapping = TableMapping::new();
        table_mapping.insert(
            tablet_id,
            TableNamespace::test_user(),
            table_number,
            table_name,
        );
        let doc = ResolvedDocument::new(
            ResolvedDocumentId::new(
                tablet_id,
                DeveloperDocumentId::new(table_number, internal_id),
            ),
            CreationTime::ONE,
            assert_obj!(
                "f" => 5
            ),
        )?;
        let mapped = doc.to_developer();
        assert_eq!(mapped.id().table(), table_number);
        Ok(())
    }

    proptest! {
        #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_document_proto_roundtrips(left in any::<ResolvedDocument>()) {
            assert_roundtrips::<ResolvedDocument, ResolvedDocumentProto>(left);
        }


        #[test]
        fn test_document_update_proto_roundtrips(left in any::<DocumentUpdateWithPrevTs>()) {
            assert_roundtrips::<DocumentUpdateWithPrevTs, DocumentUpdateWithPrevTsProto>(left);
        }


        #[test]
        fn test_index_document_update_proto_roundtrips(left in any::<DocumentUpdate>()) {
            assert_roundtrips::<DocumentUpdate, DocumentUpdateProto>(left);
        }

        #[test]
        fn test_packed_document_index_key_matches(
            id in any::<ResolvedDocumentId>(),
            creation_time in any::<CreationTime>(),
            value in any_with::<ConvexObject>((
                prop::collection::SizeRange::default(),
                FieldType::UserIdentifier,
                ValueBranching::medium(),
                ExcludeSetsAndMaps(false),
                RestrictNaNs(false),
            )),
            field_paths in prop::collection::vec(
                prop::collection::vec(
                    any::<Option<prop::sample::Index>>(),
                    1..3
                ),
                0..4
            )
        ) {
            let mut object = BTreeMap::from(value);
            object.insert(ID_FIELD.clone().into(), id.into());
            object.insert(
                CREATION_TIME_FIELD.clone().into(),
                ConvexValue::from(f64::from(creation_time)),
            );
            let value = ConvexObject::try_from(object).unwrap();
            let doc = ResolvedDocument::new(id, creation_time, value).unwrap();
            // Generate field paths that have a chance of resolving to something for `doc`
            let mut current_doc = Some(&**doc.value());
            let field_paths: Vec<_> = field_paths.into_iter().filter_map(|indexes| {
                let ids = indexes.into_iter().map(|index| {
                    if let (Some(index), Some(c)) = (index, current_doc) && !c.is_empty() {
                        let k = c.keys().nth(index.index(c.len())).unwrap().clone();
                        current_doc = c.get(&k).and_then(|x| {
                            if let ConvexValue::Object(o) = x { Some(o) } else { None }
                        });
                        k
                    } else {
                        current_doc = None;
                        "unknown".parse().unwrap()
                    }
                })
                    .filter_map(|field_name| IdentifierFieldName::from_str(&field_name).ok())
                    .collect();
                FieldPath::new(ids).ok()
            }).collect();
            let ver = PersistenceVersion::V5;
            let index_key_bytes = doc.index_key(&field_paths, ver).to_bytes();
            assert_eq!(
                index_key_bytes,
                *PackedDocument::pack(&doc).index_key(
                    &field_paths, ver, &mut IndexKeyBuffer::new()
                ),
            );
        }
    }

    #[test]
    fn test_index_key_missing_field() -> anyhow::Result<()> {
        let doc1 = ResolvedDocument::new(
            ResolvedDocumentId::MIN,
            CreationTime::ONE,
            assert_obj!(
                "_id" => DeveloperDocumentId::MIN,
                "foo" => {
                    "bar" => 5,
                    "baz" => false,
                },
            ),
        )?;
        let doc2 = ResolvedDocument::new(
            ResolvedDocumentId::MIN,
            CreationTime::ONE,
            assert_obj!(
                "_id" => DeveloperDocumentId::MIN,
                "foo" => {"bar" => 5},
            ),
        )?;
        let fields = vec![
            FieldPath::new(vec!["foo".parse()?, "bar".parse()?])?,
            FieldPath::new(vec!["foo".parse()?, "baz".parse()?])?,
        ];
        // When document has all fields for the index, index_key extracts those fields.
        assert_eq!(
            doc1.index_key(&fields[..], PersistenceVersion::default())
                .indexed_values(),
            &vec![Some(ConvexValue::from(5)), Some(ConvexValue::from(false))][..]
        );
        // When document is missing a field, assume Null.
        assert_eq!(
            doc2.index_key(&fields[..], PersistenceVersion::default())
                .indexed_values(),
            &vec![Some(ConvexValue::from(5)), None][..]
        );
        Ok(())
    }
}
