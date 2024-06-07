use std::sync::LazyLock;

use serde::{
    Deserialize,
    Serialize,
};
use value::{
    codegen_convex_serialization,
    TableNamespace,
    TableNumber,
};

use crate::{
    components::COMPONENTS_ENABLED,
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
    /// New documents may be created in this table by
    /// `ImportFacingModel::insert`. It may have the same name and/or number
    /// as an existing Active table. It appears in only one direction of
    /// TableMapping, so to find its mapping you must look it up by TableId,
    /// not TableNumber or TableName.
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
    // TODO(lee) allow any TableNamespace once they are supported in tests.
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(value = "TableNamespace::Global")
    )]
    pub namespace: TableNamespace,
}

impl TableMetadata {
    pub fn is_active(&self) -> bool {
        matches!(self.state, TableState::Active)
    }
}

impl TableMetadata {
    pub fn new(namespace: TableNamespace, name: TableName, number: TableNumber) -> Self {
        Self {
            name,
            number,
            state: TableState::Active,
            namespace,
        }
    }

    pub fn new_with_state(
        namespace: TableNamespace,
        name: TableName,
        number: TableNumber,
        state: TableState,
    ) -> Self {
        Self {
            name,
            number,
            state,
            namespace,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct SerializedTableMetadata {
    name: String,
    number: i64,
    state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    namespace: Option<SerializedTableNamespace>,
}

impl TryFrom<TableMetadata> for SerializedTableMetadata {
    type Error = anyhow::Error;

    fn try_from(m: TableMetadata) -> anyhow::Result<Self> {
        Ok(Self {
            name: m.name.into(),
            number: u32::from(m.number) as i64,
            state: match m.state {
                TableState::Active => "active".to_owned(),
                TableState::Deleting => "deleting".to_owned(),
                TableState::Hidden => "hidden".to_owned(),
            },
            namespace: table_namespace_to_serialized(m.namespace)?,
        })
    }
}

impl TryFrom<SerializedTableMetadata> for TableMetadata {
    type Error = anyhow::Error;

    fn try_from(m: SerializedTableMetadata) -> anyhow::Result<Self> {
        Ok(Self {
            name: m.name.parse()?,
            number: u32::try_from(m.number)?.try_into()?,
            state: match &m.state[..] {
                "active" => TableState::Active,
                "deleting" => TableState::Deleting,
                "hidden" => TableState::Hidden,
                s => anyhow::bail!("invalid table state {s}"),
            },
            namespace: table_namespace_from_serialized(m.namespace)?,
        })
    }
}

codegen_convex_serialization!(TableMetadata, SerializedTableMetadata);

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[serde(rename_all = "camelCase", tag = "kind")]
enum SerializedTableNamespace {
    ByComponent {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
}

fn table_namespace_from_serialized(
    m: Option<SerializedTableNamespace>,
) -> anyhow::Result<TableNamespace> {
    Ok(match m {
        None => TableNamespace::Global,
        Some(SerializedTableNamespace::ByComponent { id: Some(id) }) => {
            TableNamespace::ByComponent(id.parse()?)
        },
        Some(SerializedTableNamespace::ByComponent { id: None }) => TableNamespace::RootComponent,
    })
}

fn table_namespace_to_serialized(
    m: TableNamespace,
) -> anyhow::Result<Option<SerializedTableNamespace>> {
    anyhow::ensure!(
        *COMPONENTS_ENABLED || matches!(m, TableNamespace::Global),
        "non-global namespaces should only be serialized when components are enabled"
    );
    match m {
        TableNamespace::Global => Ok(None),
        TableNamespace::ByComponent(id) => Ok(Some(SerializedTableNamespace::ByComponent {
            id: Some(id.to_string()),
        })),
        TableNamespace::RootComponent => {
            Ok(Some(SerializedTableNamespace::ByComponent { id: None }))
        },
    }
}

#[cfg(test)]
mod tests {
    use value::{
        assert_obj,
        obj,
        ConvexObject,
        TableNamespace,
    };

    use super::TableMetadata;
    use crate::bootstrap_model::tables::TableState;

    #[test]
    fn test_backwards_compatibility() -> anyhow::Result<()> {
        let serialized = obj!(
            "name" => "foo",
            "state" => "hidden",
            "number" => 1017,
        )?;
        let deserialized: TableMetadata = serialized.try_into().unwrap();
        assert_eq!(
            deserialized,
            TableMetadata {
                name: "foo".parse()?,
                number: 1017.try_into()?,
                state: TableState::Hidden,
                namespace: TableNamespace::Global,
            }
        );
        Ok(())
    }

    #[test]
    fn test_global_namespace() -> anyhow::Result<()> {
        let table = TableMetadata {
            name: "foo".parse()?,
            number: 1017.try_into()?,
            state: TableState::Active,
            namespace: TableNamespace::Global,
        };
        let serialized: ConvexObject = table.try_into()?;
        assert_eq!(
            serialized,
            assert_obj!(
                "name" => "foo",
                "state" => "active",
                "number" => 1017,
            ),
        );
        Ok(())
    }
}
