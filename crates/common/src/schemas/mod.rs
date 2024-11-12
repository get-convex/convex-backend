use std::{
    collections::{
        BTreeMap,
        BTreeSet,
        HashSet,
    },
    fmt::Display,
    iter,
    marker::PhantomData,
};

use errors::ErrorMetadata;
use itertools::{
    Either,
    Itertools,
};
#[cfg(any(test, feature = "testing"))]
use proptest::prelude::*;
use shape_inference::{
    Shape,
    ShapeConfig,
    ShapeCounter,
};
#[cfg(any(test, feature = "testing"))]
use value::TableType;
use value::{
    id_v6::DeveloperDocumentId,
    ConvexObject,
    ConvexValue,
    IdentifierFieldName,
    Namespace,
    NamespacedTableMapping,
};

use self::validator::{
    ObjectValidator,
    ValidationError,
    Validator,
};
use crate::{
    bootstrap_model::index::{
        database_index::IndexedFields,
        index_validation_error,
        vector_index::VectorDimensions,
        MAX_TEXT_INDEX_FILTER_FIELDS_SIZE,
        MAX_VECTOR_INDEX_FILTER_FIELDS_SIZE,
    },
    document::ResolvedDocument,
    paths::FieldPath,
    types::{
        IndexDescriptor,
        TableName,
    },
    virtual_system_mapping::VirtualSystemMapping,
};

pub mod json;
#[cfg(test)]
mod tests;
pub mod validator;

pub const MAX_INDEXES_PER_TABLE: usize = 64;
#[derive(derive_more::Display, Debug, Clone, PartialEq)]
pub enum SchemaValidationError {
    #[display(
        fmt = "Document with ID \"{id}\" in table \"{table_name}\" does not match the schema: \
               {validation_error}"
    )]
    ExistingDocument {
        validation_error: ValidationError,
        table_name: TableName,
        id: DeveloperDocumentId,
    },

    // TODO: Figure out if it's possible to surface the document ID here,
    // this is a concurrent write condition
    #[display(
        fmt = "New document in table \"{table_name}\" does not match the schema: \
               {validation_error}"
    )]
    NewDocument {
        validation_error: ValidationError,
        table_name: TableName,
    },

    #[display(fmt = "Failed to delete table \"{table_name}\" because it appears in the schema")]
    TableCannotBeDeleted { table_name: TableName },
    #[display(
        fmt = "Failed to delete table \"{table_name}\" because `v.id(\"{table_name}\")` appears \
               in the schema of table \"{table_in_schema}\""
    )]
    ReferencedTableCannotBeDeleted {
        table_in_schema: TableName,
        table_name: TableName,
    },
}

#[derive(derive_more::Display, Debug, Clone, PartialEq)]
pub enum SchemaEnforcementError {
    #[display(
        fmt = "Failed to insert or update a document in table \"{table_name}\" because it does \
               not match the schema: {validation_error}"
    )]
    Document {
        validation_error: ValidationError,
        table_name: TableName,
    },
    #[display(fmt = "Failed to delete table \"{table_name}\" because it appears in the schema")]
    TableCannotBeDeleted { table_name: TableName },
    #[display(
        fmt = "Failed to delete table \"{table_name}\" because `v.id(\"{table_name}\")` appears \
               in the schema of table \"{table_in_schema}\""
    )]
    ReferencedTableCannotBeDeleted {
        table_in_schema: TableName,
        table_name: TableName,
    },
}

impl SchemaEnforcementError {
    pub fn to_error_metadata(self) -> ErrorMetadata {
        ErrorMetadata::bad_request("SchemaEnforcementError", self.to_string())
    }
}

impl From<SchemaEnforcementError> for SchemaValidationError {
    fn from(value: SchemaEnforcementError) -> Self {
        match value {
            SchemaEnforcementError::Document {
                validation_error,
                table_name,
            } => Self::NewDocument {
                validation_error,
                table_name,
            },
            SchemaEnforcementError::TableCannotBeDeleted { table_name } => {
                Self::TableCannotBeDeleted { table_name }
            },
            SchemaEnforcementError::ReferencedTableCannotBeDeleted {
                table_in_schema,
                table_name,
            } => Self::ReferencedTableCannotBeDeleted {
                table_in_schema,
                table_name,
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatabaseSchema {
    pub tables: BTreeMap<TableName, TableDefinition>,
    pub schema_validation: bool,
}

#[macro_export]
/// Create a DatabaseSchema from TableNames and DocumentSchemas.
macro_rules! db_schema {
    ($($table:expr => $document_schema:expr),* $(,)?) => {
        {
            use std::collections::BTreeMap;
            #[allow(unused)]
            use $crate::types::TableName;
            use $crate::schemas::DatabaseSchema;
            #[allow(unused)]
            let mut tables = BTreeMap::new();
            {
                $(
                    let table_name: TableName = $table.to_string().parse()?;
                    let table_def = $crate::schemas::TableDefinition {
                        table_name: table_name.clone(),
                        indexes: Default::default(),
                        search_indexes: Default::default(),
                        vector_indexes: Default::default(),
                        document_type: Some($document_schema),
                    };
                    tables.insert(table_name, table_def);
                )*
            }
            DatabaseSchema {
                tables,
                schema_validation: true,
            }
        }
    };
}

#[macro_export]
/// Creates a `[DatabaseSchema]` that is not validated.
macro_rules! db_schema_not_validated {
    ($($table:expr => $document_schema:expr),* $(,)?) => {
        {
            use std::collections::BTreeMap;
            #[allow(unused)]
            use $crate::types::TableName;
            #[allow(unused)]
            let mut tables = BTreeMap::new();
            {
                $(
                    let table_name: TableName = $table.to_string().parse()?;
                    let table_def = $crate::schemas::TableDefinition {
                        table_name: table_name.clone(),
                        indexes: Default::default(),
                        search_indexes: Default::default(),
                        vector_indexes: Default::default(),
                        document_type: Some($document_schema),
                    };
                    tables.insert(table_name, table_def);
                )*
            }
            DatabaseSchema {
                tables,
                schema_validation: false,
            }
        }
    };
}

pub const VECTOR_DIMENSIONS: u32 = 1536;

#[macro_export]
// Turns a mapping of tableName => (index_name, vector_field) into a
// DatabaseSchema struct.
macro_rules! db_schema_with_vector_indexes {
    ($($table:expr => {
        $document_schema:expr, [$(($index_name:expr, $vector_field:expr)),*]
    }),* $(,)?) => {
        {
            #[allow(unused)]
            use std::str::FromStr;
            #[allow(unused)]
            let mut tables = std::collections::BTreeMap::new();
            {
                $(
                    let table_name: $crate::types::TableName =
                        str::parse($table)?;
                    #[allow(unused)]
                    let mut vector_indexes = std::collections::BTreeMap::new();
                    $(
                        let index_name = $crate::types::IndexName::new(
                            str::parse($table)?,
                            $index_name.parse()?
                        )?;
                        vector_indexes.insert(
                            index_name.descriptor().clone(),
                            $crate::schemas::VectorIndexSchema::new(
                                index_name.descriptor().clone(),
                                value::FieldPath::from_str($vector_field)?,
                                1536u32.try_into()?,
                                Default::default(),
                            )?,
                        );
                    )*
                    let table_def = $crate::schemas::TableDefinition {
                        table_name: table_name.clone(),
                        indexes: Default::default(),
                        search_indexes: Default::default(),
                        vector_indexes,
                        document_type: Some($document_schema),
                    };
                    tables.insert(table_name, table_def);
                )*
            }
            $crate::schemas::DatabaseSchema {
                tables,
                schema_validation: true,
            }
        }
    };
}

impl DatabaseSchema {
    pub fn tables_to_validate<'a, C: ShapeConfig, S: ShapeCounter, F>(
        new_schema: &'a DatabaseSchema,
        active_schema: Option<DatabaseSchema>,
        table_mapping: &NamespacedTableMapping,
        virtual_system_mapping: &VirtualSystemMapping,
        shape_provider: &F,
    ) -> anyhow::Result<BTreeSet<&'a TableName>>
    where
        F: Fn(&TableName) -> Option<Shape<C, S>>,
    {
        if !new_schema.schema_validation {
            return Ok(BTreeSet::new());
        }

        let possible_table_names: Vec<Option<&TableName>> = new_schema
            .tables
            .iter()
            .map(|(table_name, table_definition)| {
                Self::must_revalidate_table(
                    table_name,
                    table_definition,
                    &active_schema,
                    table_mapping,
                    virtual_system_mapping,
                    &shape_provider(table_name),
                )
                .map(|must_revalidate| must_revalidate.then_some(table_name))
            })
            .try_collect()?;
        Ok(possible_table_names.into_iter().flatten().collect())
    }

    fn must_revalidate_table<C: ShapeConfig, S: ShapeCounter>(
        table_name: &TableName,
        table_definition: &TableDefinition,
        active_schema: &Option<DatabaseSchema>,
        table_mapping: &NamespacedTableMapping,
        virtual_system_mapping: &VirtualSystemMapping,
        table_shape: &Option<Shape<C, S>>,
    ) -> anyhow::Result<bool> {
        let next_schema = table_definition.document_type.clone();
        let next_schema_validator: Validator = next_schema.into();

        // Can skip validation thanks to the schema diff?
        let enforced_schema = match active_schema {
            Some(active_schema) if active_schema.schema_validation => {
                active_schema.schema_for_table(table_name).cloned()
            },
            _ => None,
        };
        let enforced_schema_validator: Validator = enforced_schema.into();
        if enforced_schema_validator.is_subset(&next_schema_validator) {
            return Ok(false);
        }

        if let Some(table_shape) = table_shape {
            // Can skip validation thanks to the saved shape?
            let validator_from_shape =
                Validator::from_shape(table_shape, table_mapping, virtual_system_mapping);
            if validator_from_shape
                .filter_top_level_system_fields()
                .is_subset(&next_schema_validator)
            {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn schema_for_table(&self, table_name: &TableName) -> Option<&DocumentSchema> {
        self.tables
            .get(table_name)
            .and_then(|table_definition| table_definition.document_type.as_ref())
    }

    fn check_value(
        &self,
        doc: &ResolvedDocument,
        table_mapping: &NamespacedTableMapping,
        virtual_system_mapping: &VirtualSystemMapping,
    ) -> Result<(), ValidationError> {
        if self.schema_validation
            && let Ok(table_name) = table_mapping.tablet_name(doc.id().tablet_id)
            && let Some(document_schema) = self.schema_for_table(&table_name)
        {
            return document_schema.check_value(
                &doc.value().0,
                table_mapping,
                virtual_system_mapping,
            );
        }
        Ok(())
    }

    pub fn check_existing_document(
        &self,
        doc: &ResolvedDocument,
        table_name: TableName,
        table_mapping: &NamespacedTableMapping,
        virtual_system_mapping: &VirtualSystemMapping,
    ) -> Result<(), SchemaValidationError> {
        self.check_value(doc, table_mapping, virtual_system_mapping)
            .map_err(|validation_error| SchemaValidationError::ExistingDocument {
                validation_error,
                table_name,
                id: doc.developer_id(),
            })
    }

    pub fn check_new_document(
        &self,
        doc: &ResolvedDocument,
        table_name: TableName,
        table_mapping: &NamespacedTableMapping,
        virtual_system_mapping: &VirtualSystemMapping,
    ) -> Result<(), SchemaEnforcementError> {
        self.check_value(doc, table_mapping, virtual_system_mapping)
            .map_err(|validation_error| SchemaEnforcementError::Document {
                validation_error,
                table_name,
            })
    }

    fn contains_table_as_reference(&self, table_name: &TableName) -> Option<TableName> {
        for table_schema in self.tables.values() {
            if let Some(document_schema) = &table_schema.document_type {
                if document_schema.foreign_keys().contains(table_name) {
                    return Some(table_schema.table_name.clone());
                }
            }
        }
        None
    }

    pub fn check_delete_table(
        &self,
        active_table_to_delete: TableName,
    ) -> Result<(), SchemaEnforcementError> {
        if self.schema_for_table(&active_table_to_delete).is_some() {
            Err(SchemaEnforcementError::TableCannotBeDeleted {
                table_name: active_table_to_delete,
            })
        } else if let Some(table_in_schema) =
            self.contains_table_as_reference(&active_table_to_delete)
        {
            Err(SchemaEnforcementError::ReferencedTableCannotBeDeleted {
                table_in_schema,
                table_name: active_table_to_delete,
            })
        } else {
            Ok(())
        }
    }

    /// Checks whether the indexes are correctly defined (if the schema is
    /// enforced, all field names referenced by indexes must exist)
    pub fn check_index_references(&self) -> anyhow::Result<()> {
        if !self.schema_validation {
            return Ok(());
        }

        for (table_name, table_definition) in &self.tables {
            if let Some((index_descriptor, field_path)) = table_definition
                .fields_referenced_in_indexes()
                .find(|(_, field_path)| {
                    table_definition
                        .document_type
                        .as_ref()
                        .map(|document_schema| !document_schema.can_contain_field(field_path))
                        .unwrap_or(false)
                })
            {
                anyhow::bail!(ErrorMetadata::bad_request(
                    "SchemaDefinitionError",
                    format!(
                        "In table \"{table_name}\" the index \"{index_descriptor}\" is invalid \
                         because it references the field {field_path} that does not exist.",
                    )
                ));
            }

            if let Some((index_descriptor, field_path)) =
                table_definition.vector_fields().find(|(_, vector_field)| {
                    !Self::is_vector_index_eligible(&table_definition.document_type, vector_field)
                })
            {
                anyhow::bail!(ErrorMetadata::bad_request(
                    "SchemaDefinitionError",
                    format!(
                        "In table \"{table_name}\" the vector index \"{index_descriptor}\" is \
                         invalid because it references the field {field_path} that is neither an \
                         array of float64 or optional array of float64.",
                    )
                ));
            }
        }

        Ok(())
    }

    fn is_vector_index_eligible(
        document_schema: &Option<DocumentSchema>,
        vector_field: &FieldPath,
    ) -> bool {
        let Some(document_schema) = document_schema else {
            // If there's no schema, hope the user knows what they're doing and
            // let them use the field.
            return true;
        };
        document_schema.is_vector_index_eligible(vector_field)
    }
}

#[cfg(any(test, feature = "testing"))]
impl Default for DatabaseSchema {
    fn default() -> Self {
        Self {
            tables: BTreeMap::new(),
            schema_validation: true,
        }
    }
}

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for DatabaseSchema {
    type Parameters = ();

    type Strategy = impl proptest::strategy::Strategy<Value = DatabaseSchema>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;

        // To generate valid schemas, first generate the set of table names.
        // In each table, only generate references to names in this set.
        (
            prop::collection::btree_set(any_with::<TableName>(TableType::User), 0..8),
            any::<bool>(),
        )
            .prop_flat_map(|(table_names, schema_validation)| {
                let cloned_names = table_names.clone();
                let table_names_and_definitions: Vec<_> = table_names
                    .into_iter()
                    .map(move |table_name| {
                        (
                            Just(table_name.clone()),
                            any_with::<TableDefinition>((table_name, cloned_names.clone())),
                        )
                    })
                    .collect();

                table_names_and_definitions.prop_map(move |names_and_defintiions| Self {
                    tables: names_and_defintiions.into_iter().collect(),
                    schema_validation,
                })
            })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableDefinition {
    pub table_name: TableName,
    pub indexes: BTreeMap<IndexDescriptor, IndexSchema>,
    pub search_indexes: BTreeMap<IndexDescriptor, SearchIndexSchema>,
    pub vector_indexes: BTreeMap<IndexDescriptor, VectorIndexSchema>,
    pub document_type: Option<DocumentSchema>,
}

impl TableDefinition {
    pub fn fields_referenced_in_indexes(
        &self,
    ) -> impl Iterator<Item = (&IndexDescriptor, &FieldPath)> {
        let index_fields = self
            .indexes
            .iter()
            .flat_map(|(index_descriptor, index_schema)| {
                index_schema
                    .fields
                    .iter()
                    .map(move |field_path| (index_descriptor, field_path))
            });

        let search_index_fields =
            self.search_indexes
                .iter()
                .map(|(index_descriptor, search_index_schema)| {
                    (index_descriptor, (&search_index_schema.search_field))
                });

        let search_index_filter_fields =
            self.search_indexes
                .iter()
                .flat_map(|(index_descriptor, search_index_schema)| {
                    search_index_schema
                        .filter_fields
                        .iter()
                        .map(move |field_path| (index_descriptor, field_path))
                });

        let vector_index_fields = self.vector_fields();

        index_fields
            .chain(search_index_fields)
            .chain(search_index_filter_fields)
            .chain(vector_index_fields)
    }

    pub fn vector_fields(&self) -> impl Iterator<Item = (&IndexDescriptor, &FieldPath)> {
        self.vector_indexes
            .iter()
            .map(|(index_descriptor, vector_index_schema)| {
                (index_descriptor, (&vector_index_schema.vector_field))
            })
    }
}

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for TableDefinition {
    type Parameters = (TableName, BTreeSet<TableName>);

    type Strategy = impl proptest::strategy::Strategy<Value = TableDefinition>;

    fn arbitrary_with((table_name, all_table_names): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;

        (
            prop::collection::vec(any::<IndexSchema>(), 0..6),
            prop::collection::vec(any::<SearchIndexSchema>(), 0..3),
            prop::collection::vec(any::<VectorIndexSchema>(), 0..3),
            any_with::<Option<DocumentSchema>>((
                prop::option::Probability::default(),
                all_table_names,
            )),
        )
            .prop_filter_map(
                "index names must be unique",
                move |(indexes, search_indexes, vector_indexes, document_type)| {
                    let index_descriptors: BTreeSet<_> = indexes
                        .iter()
                        .map(|i| &i.index_descriptor)
                        .chain(search_indexes.iter().map(|i| &i.index_descriptor))
                        .chain(vector_indexes.iter().map(|i| &i.index_descriptor))
                        .collect();
                    let expected = indexes.len() + search_indexes.len() + vector_indexes.len();
                    assert!(index_descriptors.len() <= expected);
                    if index_descriptors.len() == expected {
                        Some(Self {
                            table_name: table_name.clone(),
                            indexes: indexes
                                .into_iter()
                                .map(|i| (i.index_descriptor.clone(), i))
                                .collect(),
                            search_indexes: search_indexes
                                .into_iter()
                                .map(|i| (i.index_descriptor.clone(), i))
                                .collect(),
                            vector_indexes: vector_indexes
                                .into_iter()
                                .map(|i| (i.index_descriptor.clone(), i))
                                .collect(),
                            document_type,
                        })
                    } else {
                        None
                    }
                },
            )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct IndexSchema {
    pub index_descriptor: IndexDescriptor,
    pub fields: IndexedFields,
}

impl Display for IndexSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.index_descriptor)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct SearchIndexSchema {
    pub index_descriptor: IndexDescriptor,
    pub search_field: FieldPath,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "prop::collection::btree_set(any::<FieldPath>(), 0..8)")
    )]
    pub filter_fields: BTreeSet<FieldPath>,

    // Private field to force all creations to go through the constructor.
    _pd: PhantomData<()>,
}

impl SearchIndexSchema {
    pub fn new(
        index_descriptor: IndexDescriptor,
        search_field: FieldPath,
        filter_fields: BTreeSet<FieldPath>,
    ) -> anyhow::Result<Self> {
        if filter_fields.len() > MAX_TEXT_INDEX_FILTER_FIELDS_SIZE {
            anyhow::bail!(index_validation_error::too_many_filter_fields(
                MAX_TEXT_INDEX_FILTER_FIELDS_SIZE
            ));
        }
        Ok(Self {
            index_descriptor,
            search_field,
            filter_fields,
            _pd: PhantomData,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct VectorIndexSchema {
    pub index_descriptor: IndexDescriptor,
    pub vector_field: FieldPath,
    pub dimension: VectorDimensions,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "prop::collection::btree_set(any::<FieldPath>(), 0..8)")
    )]
    pub filter_fields: BTreeSet<FieldPath>,

    // Private field to force all creations to go through the constructor.
    _pd: PhantomData<()>,
}

impl VectorIndexSchema {
    pub fn new(
        index_descriptor: IndexDescriptor,
        vector_field: FieldPath,
        dimension: VectorDimensions,
        filter_fields: BTreeSet<FieldPath>,
    ) -> anyhow::Result<Self> {
        if filter_fields.len() > MAX_VECTOR_INDEX_FILTER_FIELDS_SIZE {
            anyhow::bail!(index_validation_error::too_many_filter_fields(
                MAX_VECTOR_INDEX_FILTER_FIELDS_SIZE
            ));
        }
        Ok(Self {
            index_descriptor,
            vector_field,
            dimension,
            filter_fields,
            _pd: PhantomData,
        })
    }
}

/// [`DocumentSchema`] corresponds to the `DocumentSchema` TS type in
/// `TableDefinition`. `Any` means no schema will be enforced.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(
    any(test, feature = "testing"),
    proptest(params = "BTreeSet<TableName>")
)]
pub enum DocumentSchema {
    Any,

    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(
            strategy = "prop::collection::vec(any_with::<ObjectValidator>(params), \
                        1..8).prop_map(DocumentSchema::Union)"
        )
    )]
    Union(Vec<ObjectValidator>),
}

impl DocumentSchema {
    fn check_value(
        &self,
        value: &ConvexObject,
        table_mapping: &NamespacedTableMapping,
        virtual_system_mapping: &VirtualSystemMapping,
    ) -> Result<(), ValidationError> {
        match self {
            DocumentSchema::Any => {},
            DocumentSchema::Union(t) => {
                let value = value.clone().filter_system_fields();
                let schema_type = t
                    .iter()
                    .map(|obj_schema| Validator::Object(obj_schema.clone()))
                    .collect();
                Validator::Union(schema_type).check_value(
                    &ConvexValue::Object(value),
                    table_mapping,
                    virtual_system_mapping,
                )?;
            },
        }
        Ok(())
    }

    /// Returns `true` when it is sometimes possible to have a field with the
    /// given path on the document if this table definition is enforced, or
    /// `false` when it is never possible.
    pub fn can_contain_field(&self, field_path: &FieldPath) -> bool {
        // Allow system fields even if they are not in the table definition
        if matches!(&field_path.fields(), [single_field] if single_field.is_system()) {
            return true;
        }

        match &self {
            DocumentSchema::Any => true,
            DocumentSchema::Union(validators) => validators.iter().any(|root_validator| {
                Validator::Object(root_validator.clone()).can_contain_field(field_path)
            }),
        }
    }

    pub fn has_validator_for_system_field(&self) -> bool {
        match &self {
            DocumentSchema::Any => false,
            DocumentSchema::Union(validators) => validators
                .iter()
                .any(|root_validator| root_validator.has_validator_for_system_field()),
        }
    }

    pub fn has_map_or_set(&self) -> bool {
        match &self {
            DocumentSchema::Any => false,
            DocumentSchema::Union(validators) => validators
                .iter()
                .any(|root_validator| root_validator.has_map_or_set()),
        }
    }

    pub fn is_vector_index_eligible(&self, field_path: &FieldPath) -> bool {
        match &self {
            DocumentSchema::Any => true,
            DocumentSchema::Union(validators) => validators.iter().any(|root_validator| {
                Validator::Object(root_validator.clone()).overlaps_with_array_float64(field_path)
            }),
        }
    }

    /// Returns the field names from top level objects in the schema that are
    /// optional.
    pub fn optional_top_level_fields(&self) -> HashSet<IdentifierFieldName> {
        match self {
            DocumentSchema::Any => HashSet::default(),
            DocumentSchema::Union(validators) => validators
                .iter()
                .flat_map(|validator| {
                    validator
                        .0
                        .iter()
                        .filter_map(|(field_name, field_validator)| {
                            if field_validator.optional {
                                Some(field_name.clone())
                            } else {
                                None
                            }
                        })
                })
                .collect(),
        }
    }

    pub fn foreign_keys(&self) -> impl Iterator<Item = &TableName> {
        match self {
            Self::Any => Either::Left(iter::empty()),
            Self::Union(options) => {
                Either::Right(options.iter().flat_map(|option| option.foreign_keys()))
            },
        }
    }
}

const SEE_SCHEMA_DOCS: &str =
    "To learn more, see the schema documentation at https://docs.convex.dev/database/schemas.";

fn invalid_top_level_type_in_schema(validator: &Validator) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "InvalidTopLevelTypeInSchemaError",
        format!(
            "The document validator in a schema must be an object, a union of objects, or \
             `v.any()`. Found {validator}. {SEE_SCHEMA_DOCS}"
        ),
    )
}

pub fn missing_schema_export_error() -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "MissingSchemaExportError",
        format!("Schema file missing default export. {SEE_SCHEMA_DOCS}"),
    )
}

pub fn invalid_schema_export_error() -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "InvalidSchemaExport",
        format!("Default export from schema file isn't a Convex schema. {SEE_SCHEMA_DOCS}"),
    )
}
