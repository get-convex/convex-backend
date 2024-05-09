use std::{
    fmt::{
        self,
        Debug,
        Display,
    },
    io::{
        self,
        Write,
    },
    ops::Deref,
    str::FromStr,
};

use byteorder::WriteBytesExt;
use derive_more::{
    Display,
    FromStr,
};
use serde::Serialize;

#[cfg(doc)]
use crate::ResolvedDocumentId;
use crate::{
    document_id::InternalId,
    field_name::FieldName,
    heap_size::HeapSize,
    identifier::{
        check_valid_identifier,
        MIN_IDENTIFIER,
    },
    sorting::{
        write_escaped_bytes,
        TERMINATOR_BYTE,
    },
    GenericDocumentId,
    Namespace,
    Size,
};

/// A table is a metadata table if and only if it has this prefix.
pub const METADATA_PREFIX: &str = "_";

/// Unique name for a table. Tables contain documents referenced by
/// [`DocumentId`]. Eventually we'll want a layer of indirection here to allow
/// users to rename their tables.
#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, derive_more::Display)]
pub struct TableName(String);

impl FromStr for TableName {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        check_valid_identifier(s)?;
        Ok(Self(s.to_owned()))
    }
}

impl Debug for TableName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Deref for TableName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<TableName> for String {
    fn from(t: TableName) -> Self {
        t.0
    }
}

impl HeapSize for TableName {
    fn heap_size(&self) -> usize {
        self.0.heap_size()
    }
}

impl TableName {
    /// Is this table in the system namespace?
    /// TODO(Rebecca): move system tables to a different namespace to avoid
    /// conflicts with virtual tables
    pub fn is_system(&self) -> bool {
        self.0.starts_with(METADATA_PREFIX)
    }

    pub fn is_valid_virtual(&self) -> bool {
        self.0.starts_with(METADATA_PREFIX)
    }

    /// Minimum valid [`TableName`]. (See the regex above.)
    pub fn min() -> Self {
        MIN_IDENTIFIER.parse().expect("Min table name invalid?")
    }
}

impl Namespace for TableName {
    fn is_system(&self) -> bool {
        self.0.starts_with(METADATA_PREFIX)
    }
}

#[cfg(any(test, feature = "testing"))]
impl TableName {
    pub fn system_strategy() -> impl proptest::strategy::Strategy<Value = TableName> {
        use crate::identifier::arbitrary_regexes::SYSTEM_IDENTIFIER_REGEX;
        SYSTEM_IDENTIFIER_REGEX.prop_filter_map("Generated invalid system TableName", |s| {
            TableName::from_str(&s).ok()
        })
    }

    pub fn user_strategy() -> impl proptest::strategy::Strategy<Value = TableName> {
        use crate::identifier::arbitrary_regexes::USER_IDENTIFIER_REGEX;
        USER_IDENTIFIER_REGEX.prop_filter_map("Generated invalid user TableName", |s| {
            TableName::from_str(&s).ok()
        })
    }
}

#[derive(Default)]
pub enum TableType {
    #[default]
    Either,
    User,
    System,
}

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for TableName {
    type Parameters = TableType;

    type Strategy = impl proptest::strategy::Strategy<Value = TableName>;

    fn arbitrary_with(ty: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        match ty {
            TableType::Either => {
                prop_oneof![TableName::system_strategy(), TableName::user_strategy(),].boxed()
            },
            TableType::User => TableName::user_strategy().boxed(),
            TableType::System => TableName::system_strategy().boxed(),
        }
    }
}

impl From<TableName> for FieldName {
    fn from(table: TableName) -> FieldName {
        table.0.parse().expect("TableName not valid FieldName")
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, FromStr, Display, Hash)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct TabletId(pub InternalId);

impl HeapSize for TabletId {
    fn heap_size(&self) -> usize {
        0
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Display, Hash, Serialize)]
pub struct TableNumber(u32);

#[cfg(any(test, feature = "testing"))]
use proptest::prelude::*;

#[cfg(any(test, feature = "testing"))]
impl Arbitrary for TableNumber {
    type Parameters = ();

    type Strategy = impl proptest::strategy::Strategy<Value = TableNumber>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        any::<u32>().prop_filter_map("Invalid table number", |x| TableNumber::try_from(x).ok())
    }
}

impl From<TableNumber> for u32 {
    fn from(n: TableNumber) -> u32 {
        n.0
    }
}

impl TryFrom<u32> for TableNumber {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        anyhow::ensure!(value > 0);
        Ok(TableNumber(value))
    }
}

impl HeapSize for TableNumber {
    fn heap_size(&self) -> usize {
        0
    }
}

impl TableIdentifier for TableNumber {
    fn min() -> Self {
        TableNumber(1)
    }

    fn write_sorted<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        write_escaped_bytes(&self.0.to_be_bytes(), writer)
    }

    fn document_id_to_string(&self, internal_id: InternalId) -> String {
        let id_v6 = GenericDocumentId::new(*self, internal_id);
        id_v6.encode()
    }
}

impl TableNumber {
    pub fn increment(self) -> anyhow::Result<Self> {
        Ok(Self(
            self.0
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("Table number overflow"))?,
        ))
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Hash)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct TabletIdAndTableNumber {
    pub table_number: TableNumber,
    pub tablet_id: TabletId,
}

impl TabletIdAndTableNumber {
    #[cfg(any(test, feature = "testing"))]
    pub fn new_for_test(tablet_id: TabletId, table_number: TableNumber) -> Self {
        TabletIdAndTableNumber {
            tablet_id,
            table_number,
        }
    }
}

impl HeapSize for TabletIdAndTableNumber {
    fn heap_size(&self) -> usize {
        0
    }
}

impl Display for TabletIdAndTableNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.table_number, f)
    }
}

impl TableIdentifier for TabletIdAndTableNumber {
    fn min() -> Self {
        Self {
            tablet_id: <TabletId as TableIdentifier>::min(),
            table_number: <TableNumber as TableIdentifier>::min(),
        }
    }

    fn write_sorted<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.table_number.write_sorted(writer)
    }

    fn document_id_to_string(&self, internal_id: InternalId) -> String {
        self.table_number.document_id_to_string(internal_id)
    }
}

// This is a Never type, which we use in `ConvexValue` to prevent
// construction of `ConvexValue::Id`
#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash, PartialOrd, Ord)]
pub enum NeverTable {}

impl HeapSize for NeverTable {
    fn heap_size(&self) -> usize {
        0
    }
}

impl Display for NeverTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt("NeverTable", f)
    }
}

impl TableIdentifier for NeverTable {
    fn min() -> Self {
        panic!("Called min on NeverTable")
    }

    fn write_sorted<W: Write>(&self, _writer: &mut W) -> io::Result<()> {
        panic!("Called write_sorted on NeverTable")
    }

    fn document_id_to_string(&self, _internal_id: InternalId) -> String {
        panic!("Called document_id_to_string on NeverTable")
    }
}

pub trait TableIdentifier:
    Debug + Display + Clone + HeapSize + Size + Ord + Eq + Sync + Send + 'static
{
    fn min() -> Self;

    /// Create a [`DocumentId`] for a given table.
    fn id(&self, internal_id: InternalId) -> GenericDocumentId<Self> {
        GenericDocumentId::new(self.clone(), internal_id)
    }

    fn write_sorted<W: Write>(&self, writer: &mut W) -> io::Result<()>;

    fn document_id_to_string(&self, internal_id: InternalId) -> String;
}

impl<T: TableIdentifier> Size for T {
    fn size(&self) -> usize {
        // In order to compute size consistently for both DocumentId<TableName> and
        // DocumentId<TableId> so it represents the size as stored in persistence,
        // assume that the size is the maximum internal id size.
        InternalId::MAX_SIZE
    }

    fn nesting(&self) -> usize {
        0
    }
}

impl TableIdentifier for TabletId {
    fn min() -> Self {
        TabletId(InternalId::MIN)
    }

    fn write_sorted<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        write_escaped_bytes(&self.0[..], writer)
    }

    fn document_id_to_string(&self, internal_id: InternalId) -> String {
        format!("{}|{}", *self, internal_id)
    }
}

impl TableIdentifier for TableName {
    fn min() -> Self {
        TableName::min()
    }

    fn write_sorted<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        // Manually write the table name as a byte string terminated with
        // TERMINATOR_BYTE.

        // This is almost the same as `write_escaped_string` except that we don't
        // escape TERMINATOR_BYTE.
        // This is because the InternalId could start with `ESCAPE_BYTE` which would
        // be ambiguous. This works because table names have a restricted
        // characterset that doesn't include TERMINATOR_BYTE.

        for &byte in format!("{}", self).as_bytes() {
            writer.write_u8(byte)?;
            if byte == TERMINATOR_BYTE {
                assert_ne!(
                    byte, TERMINATOR_BYTE,
                    "Table name {} contains a null byte",
                    self
                );
            }
        }
        Ok(())
    }

    fn document_id_to_string(&self, internal_id: InternalId) -> String {
        format!("{}|{}", self.clone(), internal_id)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        InternalId,
        TableName,
        TableNumber,
        TabletId,
        TabletIdAndTableNumber,
    };

    #[test]
    fn table_name_valid() {
        assert!("hello_world".parse::<TableName>().is_ok());
        assert!("one_two_three_four_five".parse::<TableName>().is_ok());
        assert!("alpha_num3r1c".parse::<TableName>().is_ok());
    }

    #[test]
    fn table_name_invalid() {
        assert!("one_tw!o_three_four_five".parse::<TableName>().is_err());
        assert!("_____".parse::<TableName>().is_err());
        assert!("".parse::<TableName>().is_err());
        assert!("sujays_edgè_cäsê".parse::<TableName>().is_err());
    }

    #[test]
    fn table_name_is_system() -> anyhow::Result<()> {
        assert!("_hello_world".parse::<TableName>()?.is_system());
        assert!("_elephant3".parse::<TableName>()?.is_system());
        assert!(!"elephant3".parse::<TableName>()?.is_system());
        Ok(())
    }

    #[test]
    fn test_tablet_id_and_table_number_cmp() {
        let id1 = TabletIdAndTableNumber {
            table_number: TableNumber(1),
            tablet_id: TabletId(InternalId::MIN),
        };
        let id2 = TabletIdAndTableNumber {
            table_number: TableNumber(1),
            tablet_id: TabletId(InternalId::MAX),
        };
        let id3 = TabletIdAndTableNumber {
            table_number: TableNumber(2),
            tablet_id: TabletId(InternalId::MIN),
        };
        let id4 = TabletIdAndTableNumber {
            table_number: TableNumber(2),
            tablet_id: TabletId(InternalId::MAX),
        };
        // Not equal despite same table number.
        assert_ne!(id1, id2);
        // Not equal despite same table id.
        assert_ne!(id1, id3);
        // Ordered by table number first.
        assert!(id1 < id2 && id2 < id3 && id3 < id4);
    }
}
