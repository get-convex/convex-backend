use std::{
    collections::BTreeSet,
    sync::Arc,
};

use common::{
    bootstrap_model::schema::SchemaState,
    runtime::Runtime,
    schemas::DatabaseSchema,
    types::TableName,
};
use database::{
    SchemaModel,
    TableModel,
    Transaction,
    SCHEMAS_TABLE,
};
use errors::ErrorMetadata;
use value::{
    ResolvedDocumentId,
    TableMapping,
    TableNamespace,
    TableNumber,
};

/// The case where a schema can become invalid:
/// 1. import is changing the table number of table "foo".
/// 2. import does not touch table "bar".
/// 3. "bar" has a foreign reference to "foo", validated by schema.
/// 4. when the import commits, "bar" is nonempty.
///
/// To prevent this case we throw an error if a schema'd table outside the
/// import is nonempty and points into the import, and the import changes the
/// table number.
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct ImportSchemaTableConstraint {
    namespace: TableNamespace,
    // "foo" in example above.
    foreign_ref_table_in_import: (TableName, TableNumber),
    // "bar" in example above.
    table_in_schema_not_in_import: TableName,
}

impl ImportSchemaTableConstraint {
    async fn validate<RT: Runtime>(&self, tx: &mut Transaction<RT>) -> anyhow::Result<()> {
        let existing_table_mapping = tx.table_mapping();
        let Some(existing_table) = existing_table_mapping
            .namespace(self.namespace)
            .id_and_number_if_exists(&self.foreign_ref_table_in_import.0)
        else {
            // If a table doesn't have a table number,
            // schema validation for foreign references into the table is
            // meaningless.
            return Ok(());
        };
        if existing_table.table_number == self.foreign_ref_table_in_import.1 {
            // The import isn't changing the table number, so the schema
            // is still valid.
            return Ok(());
        }
        if TableModel::new(tx)
            .must_count(self.namespace, &self.table_in_schema_not_in_import)
            .await?
            == 0
        {
            // Schema is validating an empty table which is meaningless.
            // We can change the table numbers without violating schema.
            return Ok(());
        }
        anyhow::bail!(ErrorMetadata::bad_request(
            "ImportForeignKey",
            format!(
                "Import changes table '{}' which is referenced by '{}' in the schema",
                self.foreign_ref_table_in_import.0, self.table_in_schema_not_in_import,
            ),
        ));
    }
}

#[derive(Clone, Debug)]
pub struct ImportSchemaConstraints {
    initial_schemas: SchemasForImport,
    table_constraints: BTreeSet<ImportSchemaTableConstraint>,
}

impl ImportSchemaConstraints {
    pub fn new(table_mapping_for_import: &TableMapping, initial_schemas: SchemasForImport) -> Self {
        let mut table_constraints = BTreeSet::new();
        for (namespace, _, (_, schema)) in initial_schemas.iter() {
            for (table, table_schema) in &schema.tables {
                if table_mapping_for_import
                    .namespace(*namespace)
                    .name_exists(table)
                {
                    // Schema's table is in the import => it's valid.
                    continue;
                }
                let Some(document_schema) = &table_schema.document_type else {
                    continue;
                };
                for foreign_key_table in document_schema.foreign_keys() {
                    if let Some(foreign_key_table_number) = table_mapping_for_import
                        .namespace(*namespace)
                        .id_and_number_if_exists(foreign_key_table)
                    {
                        table_constraints.insert(ImportSchemaTableConstraint {
                            namespace: *namespace,
                            table_in_schema_not_in_import: table.clone(),
                            foreign_ref_table_in_import: (
                                foreign_key_table.clone(),
                                foreign_key_table_number.table_number,
                            ),
                        });
                    }
                }
            }
        }
        Self {
            initial_schemas,
            table_constraints,
        }
    }

    pub async fn validate<RT: Runtime>(&self, tx: &mut Transaction<RT>) -> anyhow::Result<()> {
        let final_schemas = schemas_for_import(tx).await?;
        anyhow::ensure!(
            self.initial_schemas == final_schemas,
            ErrorMetadata::bad_request(
                "ImportSchemaChanged",
                "Could not complete import because schema changed. Avoid modifying schema.ts \
                 while importing tables",
            )
        );
        for table_constraint in self.table_constraints.iter() {
            table_constraint.validate(tx).await?;
        }
        Ok(())
    }
}

pub type SchemasForImport = Vec<(
    TableNamespace,
    SchemaState,
    (ResolvedDocumentId, Arc<DatabaseSchema>),
)>;

/// Documents in an imported table should match the schema.
/// ImportFacingModel::insert checks that new documents match the schema,
/// but SchemaWorker does not check new schemas against existing documents in
/// Hidden tables. This is useful if the import fails and a Hidden table is left
/// dangling, because it should not block new schemas.
/// So, to avoid a race condition where the schema changes *during* an import
/// and SchemaWorker says the schema is valid without checking the partially
/// imported documents, we make sure all relevant schemas stay the same.
pub async fn schemas_for_import<RT: Runtime>(
    tx: &mut Transaction<RT>,
) -> anyhow::Result<SchemasForImport> {
    let mut namespaces = tx.table_mapping().namespaces_for_name(&SCHEMAS_TABLE);
    namespaces.sort();
    let mut schemas = vec![];
    for namespace in namespaces {
        let mut schema_model = SchemaModel::new(tx, namespace);
        for schema_state in [
            SchemaState::Active,
            SchemaState::Validated,
            SchemaState::Pending,
        ] {
            if let Some(schema) = schema_model.get_by_state(schema_state.clone()).await? {
                schemas.push((namespace, schema_state, schema));
            }
        }
    }
    Ok(schemas)
}
