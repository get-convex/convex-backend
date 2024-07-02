use common::bootstrap_model::tables::{
    table_namespace_from_serialized,
    table_namespace_to_serialized,
    SerializedTableNamespace,
};
use serde::{
    Deserialize,
    Serialize,
};
use value::{
    codegen_convex_serialization,
    TableName,
    TableNamespace,
    TableNumber,
};

#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct VirtualTableMetadata {
    pub name: TableName,
    pub number: TableNumber,
    // TODO(lee) allow any TableNamespace once they are supported in tests.
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(value = "TableNamespace::Global")
    )]
    pub namespace: TableNamespace,
}

impl VirtualTableMetadata {
    pub fn new(namespace: TableNamespace, name: TableName, number: TableNumber) -> Self {
        Self {
            name,
            number,
            namespace,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SerializedVirtualTableMetadata {
    name: String,
    number: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    namespace: Option<SerializedTableNamespace>,
}

impl TryFrom<VirtualTableMetadata> for SerializedVirtualTableMetadata {
    type Error = anyhow::Error;

    fn try_from(value: VirtualTableMetadata) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name.to_string(),
            number: u32::from(value.number) as i64,
            namespace: table_namespace_to_serialized(value.namespace)?,
        })
    }
}

impl TryFrom<SerializedVirtualTableMetadata> for VirtualTableMetadata {
    type Error = anyhow::Error;

    fn try_from(value: SerializedVirtualTableMetadata) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name.parse()?,
            number: u32::try_from(value.number)?.try_into()?,
            namespace: table_namespace_from_serialized(value.namespace)?,
        })
    }
}

codegen_convex_serialization!(VirtualTableMetadata, SerializedVirtualTableMetadata);
