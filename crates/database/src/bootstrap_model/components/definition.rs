use std::sync::LazyLock;

use common::{
    bootstrap_model::components::definition::ComponentDefinitionMetadata,
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
};
use value::TableName;

use crate::defaults::{
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
    fn table_name(&self) -> &'static TableName {
        &COMPONENT_DEFINITIONS_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        Vec::new()
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParsedDocument::<ComponentDefinitionMetadata>::try_from(document)?;
        Ok(())
    }
}
