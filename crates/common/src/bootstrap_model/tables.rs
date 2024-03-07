use std::{
    collections::BTreeMap,
    sync::LazyLock,
};

use value::{
    ConvexObject,
    ConvexValue,
    TableNumber,
};

use crate::{
    obj,
    types::{
        FieldName,
        TableName,
    },
};

pub static TABLES_TABLE: LazyLock<TableName> =
    LazyLock::new(|| "_tables".parse().expect("Invalid built-in tables table"));

pub static NAME_FIELD: LazyLock<FieldName> =
    LazyLock::new(|| "name".parse().expect("Invalid name field"));

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum TableState {
    /// The table exists. It was created and has not been deleted.
    Active,
    /// This table is in the process of being imported with snapshot import.
    /// New documents may be created in this table by `insert_for_import`.
    /// It may have the same name and/or number as an existing Active table.
    /// It appears in only one direction of TableMapping, so to find its mapping
    /// you must look it up by TableId, not TableNumber or TableName.
    Hidden,
    /// The table has been marked as deleted. Documents in the table may still
    /// exist, but they should be ignored.
    /// No new documents may be created in the table.
    /// A handful of legacy tables were not marked as Deleting but instead their
    /// _tables entry was deleted. Such tables should be treated the same as
    /// Deleting -- for now. Eventually we may want to clean up Deleting tables
    /// and delete the _tables rows or create a new Deleted state.
    Deleting,
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct TableMetadata {
    pub name: TableName,
    pub number: TableNumber,
    pub state: TableState,
}

impl TableMetadata {
    pub fn is_active(&self) -> bool {
        matches!(self.state, TableState::Active)
    }
}

impl TableMetadata {
    pub fn new(name: TableName, number: TableNumber) -> Self {
        Self {
            name,
            number,
            state: TableState::Active,
        }
    }

    pub fn new_with_state(name: TableName, number: TableNumber, state: TableState) -> Self {
        Self {
            name,
            number,
            state,
        }
    }
}

impl TryFrom<TableMetadata> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: TableMetadata) -> Result<Self, Self::Error> {
        obj!(
            "name" => String::from(value.name),
            "state" => String::from(match value.state {
                TableState::Active => "active",
                TableState::Deleting => "deleting",
                TableState::Hidden => "hidden",
            }),
            "number" => ConvexValue::Int64(u32::from(value.number).into()),
        )
    }
}

impl TryFrom<ConvexObject> for TableMetadata {
    type Error = anyhow::Error;

    fn try_from(object: ConvexObject) -> Result<Self, Self::Error> {
        let mut fields: BTreeMap<_, _> = object.into();
        let name = match fields.remove(&*NAME_FIELD) {
            Some(ConvexValue::String(s)) => s.parse()?,
            v => anyhow::bail!("Invalid name field for TableMetadata: {:?}", v),
        };

        let number = match fields.remove("number") {
            Some(ConvexValue::Int64(v)) => u32::try_from(v)?.try_into()?,
            v => anyhow::bail!("Invalid number field for TableMetadata: {:?}", v),
        };
        let state = match fields.remove("state") {
            Some(ConvexValue::String(s)) => match &s[..] {
                "active" => TableState::Active,
                "deleting" => TableState::Deleting,
                "hidden" => TableState::Hidden,
                _ => anyhow::bail!("invalid table state {s}"),
            },
            None => TableState::Active,
            _ => anyhow::bail!("invalid table state {fields:?}"),
        };
        Ok(Self {
            name,
            number,
            state,
        })
    }
}

impl TryFrom<ConvexValue> for TableMetadata {
    type Error = anyhow::Error;

    fn try_from(value: ConvexValue) -> Result<Self, Self::Error> {
        match value {
            ConvexValue::Object(o) => o.try_into(),
            _ => anyhow::bail!("Invalid table metadata value"),
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use value::ConvexObject;

    use super::TableMetadata;
    use crate::testing::assert_roundtrips;

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_table_roundtrips(v in any::<TableMetadata>()) {
            assert_roundtrips::<TableMetadata, ConvexObject>(v);
        }
    }
}
