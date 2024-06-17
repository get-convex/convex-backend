use errors::ErrorMetadata;
use sync_types::{
    ModulePath,
    UdfPath,
};
use value::{
    id_v6::DeveloperDocumentId,
    NamespacedTableMapping,
    ResolvedDocumentId,
    TableName,
};

pub fn parse_udf_path(path: &str) -> anyhow::Result<UdfPath> {
    path.parse().map_err(|e: anyhow::Error| {
        let msg = format!("{path} is not a valid path to a Convex function. {e}");
        e.context(ErrorMetadata::bad_request(
            "BadConvexFunctionIdentifier",
            msg,
        ))
    })
}

pub fn parse_module_path(path: &str) -> anyhow::Result<ModulePath> {
    path.parse().map_err(|e: anyhow::Error| {
        let msg = format!("{path} is not a valid path to a Convex module. {e}");
        e.context(ErrorMetadata::bad_request("BadConvexModuleIdentifier", msg))
    })
}

pub fn invalid_id_error(table_name: &TableName) -> ErrorMetadata {
    ErrorMetadata::bad_request("InvalidId", format!("Invalid ID for table {}", table_name))
}

/// Parse a string in the format of IDv6 into a [`ResolvedDocumentId`].
pub fn parse_document_id(
    id: &str,
    table_mapping: &NamespacedTableMapping,
    table_name: &TableName,
) -> anyhow::Result<ResolvedDocumentId> {
    let id = DeveloperDocumentId::decode(id)?.to_resolved(&table_mapping.number_to_tablet())?;
    anyhow::ensure!(
        table_mapping.tablet_matches_name(id.tablet_id, table_name),
        invalid_id_error(table_name)
    );
    Ok(id)
}

#[cfg(test)]
mod tests {
    use common::testing::TestIdGenerator;
    use model::environment_variables::ENVIRONMENT_VARIABLES_TABLE;
    use value::{
        id_v6::DeveloperDocumentId,
        TableNamespace,
    };

    use super::parse_document_id;

    #[test]
    fn test_parse_idv5_or_idv6() -> anyhow::Result<()> {
        let mut id_generator = TestIdGenerator::new();

        let id_v5 = id_generator.system_generate(&ENVIRONMENT_VARIABLES_TABLE);
        let id_v6: DeveloperDocumentId = id_v5.into();

        let table_mapping = id_generator.namespace(TableNamespace::Global);
        let id_v6_string = id_v6.encode();
        parse_document_id(&id_v6_string, &table_mapping, &ENVIRONMENT_VARIABLES_TABLE)?;
        Ok(())
    }
}
