use std::sync::LazyLock;

use common::bootstrap_model::components::definition::ComponentDefinitionMetadata;
use value::TableName;

use crate::system_tables::{
    SystemIndex,
    SystemTable,
};

pub static COMPONENT_DEFINITIONS_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_component_definitions"
        .parse()
        .expect("Invalid built-in _component_definitions table")
});

pub struct ComponentDefinitionsTable;

impl SystemTable for ComponentDefinitionsTable {
    type Metadata = ComponentDefinitionMetadata;

    fn table_name() -> &'static TableName {
        &COMPONENT_DEFINITIONS_TABLE
    }

    fn indexes() -> Vec<SystemIndex<Self>> {
        Vec::new()
    }
}
