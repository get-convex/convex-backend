use std::{
    fmt::{
        self,
        Debug,
    },
    ops::Deref,
    str::FromStr,
};

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
    DeveloperDocumentId,
    Namespace,
    Size,
};

/// A table is a metadata table if and only if it has this prefix.
pub const METADATA_PREFIX: &str = "_";

/// Unique name for a table. Tables contain documents referenced by
/// [`ResolvedDocumentId`]. Eventually we'll want a layer of indirection here to
/// allow users to rename their tables.
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

impl TabletId {
    pub const MIN: TabletId = TabletId(InternalId::MIN);

    pub fn document_id_to_string(&self, internal_id: InternalId) -> String {
        format!("{}|{}", *self, internal_id)
    }
}

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

impl TableNumber {
    pub const MIN: TableNumber = TableNumber(1);

    pub fn document_id_to_string(&self, internal_id: InternalId) -> String {
        let id_v6 = DeveloperDocumentId::new(*self, internal_id);
        id_v6.encode()
    }

    pub fn increment(self) -> anyhow::Result<Self> {
        Ok(Self(
            self.0
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("Table number overflow"))?,
        ))
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct TabletIdAndTableNumber {
    pub table_number: TableNumber,
    pub tablet_id: TabletId,
}

impl Size for TableNumber {
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

impl Size for TabletId {
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

#[cfg(test)]
mod tests {
    use super::TableName;

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
}
