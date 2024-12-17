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
    PackedValue,
};
use pb::common::{
    DocumentUpdate as DocumentUpdateProto,
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
    index::IndexKey,
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
    creation_time: Option<CreationTime>,
    value: PII<ConvexObject>,
}

impl DeveloperDocument {
    pub fn creation_time(&self) -> Option<CreationTime> {
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

impl From<DeveloperDocument> for JsonValue {
    fn from(doc: DeveloperDocument) -> JsonValue {
        doc.into_value().0.into()
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
        let value = serde_json::to_vec(&JsonValue::from(value.0))?;
        let id = ResolvedDocumentId {
            tablet_id,
            developer_id: id,
        };
        Ok(Self {
            id: Some(id.into()),
            creation_time: creation_time.map(|t| t.into()),
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
        let creation_time = creation_time.map(|t| t.try_into()).transpose()?;
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
    fn new_internal(
        id: ResolvedDocumentId,
        creation_time: Option<CreationTime>,
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
        let creation_time_value = creation_time.map(|t| ConvexValue::from(f64::from(t)));
        match (
            creation_time_value,
            value.get(&FieldName::from(CREATION_TIME_FIELD.clone())),
        ) {
            (Some(time), None) => {
                let mut fields: BTreeMap<_, _> = value.into();
                fields.insert(CREATION_TIME_FIELD.to_owned().into(), time);
                value = fields.try_into()?;
            },
            (l, r) if l.as_ref() == r => (),
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

    pub fn new(
        id: ResolvedDocumentId,
        creation_time: CreationTime,
        value: ConvexObject,
    ) -> anyhow::Result<Self> {
        Self::new_internal(id, Some(creation_time), value)
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
            Some(ConvexValue::Float64(ts)) => Some((*ts).try_into()?),
            None => None,
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
            Some(ConvexValue::Float64(ts)) => Some((*ts).try_into()?),
            None => None,
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
        Self::new_internal(self.id(), self.creation_time, new_value)
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
pub struct DocumentUpdate {
    pub id: ResolvedDocumentId,
    pub old_document: Option<ResolvedDocument>,
    pub new_document: Option<ResolvedDocument>,
}

impl HeapSize for DocumentUpdate {
    fn heap_size(&self) -> usize {
        self.old_document.heap_size() + self.new_document.heap_size()
    }
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

impl DeveloperDocument {
    pub fn new(
        id: DeveloperDocumentId,
        creation_time: Option<CreationTime>,
        value: ConvexObject,
    ) -> Self {
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
pub struct PackedDocument(PackedValue<ByteBuffer>, ResolvedDocumentId);

impl PackedDocument {
    pub fn pack(document: ResolvedDocument) -> Self {
        let document_id = document.id();
        let value = document.document.into_value().0.into();
        Self(PackedValue::pack(&value), document_id)
    }

    pub fn unpack(&self) -> ResolvedDocument {
        let value = ConvexValue::try_from(self.0.clone()).expect("Couldn't unpack packed value");
        let document_id = self.1;
        ResolvedDocument::from_packed(value, document_id)
            .expect("Packed value wasn't a valid document?")
    }

    /// Same behavior as ResolvedDocument::id but you don't have to fully
    /// unpack.
    pub fn id(&self) -> ResolvedDocumentId {
        self.1
    }

    pub fn value(&self) -> &PackedValue<ByteBuffer> {
        &self.0
    }

    /// Same behavior as ResolvedDocument::index_key but you don't have to fully
    /// unpack.
    pub fn index_key(
        &self,
        fields: &[FieldPath],
        _persistence_version: PersistenceVersion,
    ) -> IndexKey {
        let mut values = vec![];
        for field in fields.iter() {
            if let Some(v) = self.0.get_path(field) {
                values.push(Some(v));
            } else {
                values.push(None);
            }
        }
        IndexKey::new_allow_missing(values, self.id().into())
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
    creation_time: Option<CreationTime>,
    value: D,
}

impl<D> ParsedDocument<D> {
    pub fn id(&self) -> ResolvedDocumentId {
        self.id
    }

    pub fn developer_id(&self) -> DeveloperDocumentId {
        self.id.into()
    }

    pub fn creation_time(&self) -> Option<CreationTime> {
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

impl<D> TryFrom<ResolvedDocument> for ParsedDocument<D>
where
    D: TryFrom<ConvexObject, Error = anyhow::Error>,
{
    type Error = anyhow::Error;

    fn try_from(document: ResolvedDocument) -> anyhow::Result<Self> {
        let id = document.id();
        let creation_time = document.creation_time;
        let value: D = document
            .document
            .into_value()
            .0
            .try_into()
            .with_context(|| format!("Failed to parse document id: {id}"))?;
        Ok(Self {
            id,
            creation_time,
            value,
        })
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
        use value::ExcludeSetsAndMaps;
        any_with::<(ResolvedDocumentId, CreationTime, ConvexObject)>((
            (),
            (),
            (
                prop::collection::SizeRange::default(),
                FieldType::User,
                ExcludeSetsAndMaps(false),
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
                let doc = ResolvedDocument::new_internal(id, Some(creation_time), value);
                doc.ok()
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use std::assert_eq;

    use cmd_util::env::env_config;
    use proptest::prelude::*;
    use sync_types::testing::assert_roundtrips;
    use value::{
        id_v6::DeveloperDocumentId,
        ConvexValue,
        InternalId,
        ResolvedDocumentId,
        TableMapping,
        TableName,
        TableNamespace,
        TableNumber,
        TabletId,
    };

    use super::{
        DocumentUpdateProto,
        ResolvedDocumentProto,
    };
    use crate::{
        assert_obj,
        document::{
            CreationTime,
            DocumentUpdate,
            ResolvedDocument,
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
            table_name.clone(),
        );
        let doc = ResolvedDocument::new_internal(
            ResolvedDocumentId::new(
                tablet_id,
                DeveloperDocumentId::new(table_number, internal_id),
            ),
            Some(CreationTime::ONE),
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
        fn test_document_update_proto_roundtrips(left in any::<DocumentUpdate>()) {
            assert_roundtrips::<DocumentUpdate, DocumentUpdateProto>(left);
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
