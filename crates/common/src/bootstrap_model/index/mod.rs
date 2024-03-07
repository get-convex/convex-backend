//! Index metadata.
pub mod database_index;
pub mod search_index;
pub mod vector_index;

use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    convert::{
        TryFrom,
        TryInto,
    },
    fmt::Debug,
    sync::LazyLock,
};

use value::{
    ConvexObject,
    ConvexValue,
    FieldName,
    IdentifierFieldName,
    TableId,
    TableIdAndTableNumber,
    TableIdentifier,
};

use self::{
    database_index::{
        DatabaseIndexState,
        DeveloperDatabaseIndexConfig,
    },
    search_index::{
        DeveloperSearchIndexConfig,
        SearchIndexState,
    },
    vector_index::{
        DeveloperVectorIndexConfig,
        VectorDimensions,
        VectorIndexSnapshotData,
        VectorIndexState,
    },
};
use crate::{
    bootstrap_model::index::vector_index::VectorIndexBackfillState,
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    obj,
    paths::FieldPath,
    types::{
        GenericIndexName,
        IndexDescriptor,
        TableName,
    },
};

/// Table name for Index data.
pub static INDEX_TABLE: LazyLock<TableName> =
    LazyLock::new(|| "_index".parse().expect("Invalid built-in index table"));
/// Field for an indexed table's name in `IndexMetadata`.
pub static TABLE_ID_FIELD_NAME: LazyLock<IdentifierFieldName> =
    LazyLock::new(|| "table_id".parse().expect("Invalid built-in field"));
pub static TABLE_ID_FIELD_PATH: LazyLock<FieldPath> =
    LazyLock::new(|| FieldPath::new(vec![TABLE_ID_FIELD_NAME.clone()]).unwrap());

pub const MAX_INDEX_FIELDS_SIZE: usize = 16;
pub const MAX_SEARCH_INDEX_FILTER_FIELDS_SIZE: usize = 16;
pub const MAX_VECTOR_INDEX_FILTER_FIELDS_SIZE: usize = 16;

pub mod index_validation_error {
    use errors::ErrorMetadata;
    use value::{
        TableIdentifier,
        TableName,
    };

    use crate::{
        paths::FieldPath,
        schemas::IndexSchema,
        types::IndexDescriptor,
    };

    pub fn empty_index(table_name: &TableName, index: &IndexSchema) -> ErrorMetadata {
        ErrorMetadata::bad_request(
            "EmptyIndex",
            format!("In table \"{table_name}\" index \"{index}\" must have at least one field."),
        )
    }
    pub fn fields_not_unique_within_index(field: &FieldPath) -> ErrorMetadata {
        ErrorMetadata::bad_request(
            "FieldsNotUniqueWithinIndex",
            format!("Duplicate field {field}. Index fields must be unique within an index."),
        )
    }
    pub fn index_not_unique(
        table_name: &TableName,
        index1: &IndexDescriptor,
        index2: &IndexDescriptor,
    ) -> ErrorMetadata {
        ErrorMetadata::bad_request(
            "IndexNotUnique",
            format!(
                "In table \"{table_name}\" index \"{index1}\" and index \"{index2}\" have the \
                 same fields. Indexes must be unique within a table."
            ),
        )
    }
    // IndexFieldsContainId is a more specific version of
    // IndexFieldNameReserved. It provides a more actionable error
    // message.
    pub fn fields_contain_id() -> ErrorMetadata {
        ErrorMetadata::bad_request(
            "IndexFieldsContainId",
            "`_id` is not a valid index field. To load documents by ID, use `db.get(id)`.",
        )
    }
    // IndexFieldsContainCreationTime is a more specific version of
    // IndexFieldNameReserved. It provides a more actionable error message.
    pub fn fields_contain_creation_time() -> ErrorMetadata {
        ErrorMetadata::bad_request("IndexFieldsContainCreationTime", 
     "`_creationTime` is automatically added to the end of each index. It should not \
        be added explicitly in the index definition. See https://docs.convex.dev/using/indexes \
        for more details."
        )
    }
    pub fn field_name_reserved() -> ErrorMetadata {
        ErrorMetadata::bad_request(
            "IndexFieldNameReserved",
            "Reserved fields (starting with `_`) are not allowed in indexes.",
        )
    }
    pub fn search_field_not_unique(
        table_name: &TableName,
        index1: &IndexDescriptor,
        index2: &IndexDescriptor,
    ) -> ErrorMetadata {
        ErrorMetadata::bad_request(
            "SearchIndexFieldNotUnique",
            format!(
                "In table \"{table_name}\" search index \"{index1}\" and search index \
                 \"{index2}\" have the same `searchField`. Search index fields must be unique \
                 within a table. You should combine the
             indexes with the same `searchField` into one index containing all `filterField`s and \
                 then use different subsets of the `filterField`s at query time."
            ),
        )
    }
    pub fn vector_field_not_unique(
        table_name: &TableName,
        index1: &IndexDescriptor,
        index2: &IndexDescriptor,
    ) -> ErrorMetadata {
        ErrorMetadata::bad_request(
            "VectorIndexFieldNotUnique",
            format!(
                "In table \"{table_name}\" vector index \"{index1}\" and vector index \
                 \"{index2}\" have the same `vectorField`. Vector index fields must be unique \
                 within a table. You should combine the
             indexes with the same `vectorField` into one index containing all `filterField`s and \
                 then use different subsets of the `filterField`s at query time."
            ),
        )
    }
    pub fn name_reserved<T: TableIdentifier>(
        table_name: &T,
        name: &IndexDescriptor,
    ) -> ErrorMetadata {
        ErrorMetadata::bad_request(
            "IndexNameReserved",
            format!(
                "In table \"{table_name}\" cannot name an index \"{name}\" because the name is \
                 reserved. Indexes may not start with an underscore or be named \"by_id\" or \
                 \"by_creation_time\"."
            ),
        )
    }
    pub fn names_not_unique(table_name: &TableName, index: &IndexDescriptor) -> ErrorMetadata {
        ErrorMetadata::bad_request(
            "IndexNamesNotUnique",
            format!("Table \"{table_name}\" has two or more definitions of index \"{index}\"."),
        )
    }
    pub fn invalid_index_name(descriptor: &str) -> ErrorMetadata {
        ErrorMetadata::bad_request(
            "InvalidIndexName",
            format!(
                "Invalid index name: \"{descriptor}\". Identifiers must be 64 characters or less, \
                 start with a letter, and only contain letters, digits, underscores."
            ),
        )
    }
    pub fn invalid_index_field(descriptor: &IndexDescriptor, field: &str) -> ErrorMetadata {
        ErrorMetadata::bad_request(
            "InvalidIndexField",
            format!("In index \"{descriptor}\": Invalid index field: \"{field}\""),
        )
    }

    // TODO - move elsewhere (near table names) - it's not indexing related
    pub fn invalid_table_name(table_name: &str) -> ErrorMetadata {
        ErrorMetadata::bad_request(
            "InvalidTableName",
            format!(
                "Invalid table name: \"{table_name}\". Identifiers must start with a letter and \
                 can only contain letters, digits, and underscores."
            ),
        )
    }
    pub fn not_enough_name_components(index_name: &str) -> ErrorMetadata {
        ErrorMetadata::bad_request(
            "IndexNotEnoughNameComponents",
            format!("Insufficient components in index name {index_name}"),
        )
    }
    pub fn too_many_fields(num_fields: usize) -> ErrorMetadata {
        ErrorMetadata::bad_request(
            "IndexTooManyFields",
            format!("Indexes may have up to {num_fields} fields."),
        )
    }
    pub fn too_many_filter_fields(num_fields: usize) -> ErrorMetadata {
        ErrorMetadata::bad_request(
            "IndexTooManyFilterFields",
            format!("Search indexes may have up to {num_fields} filter fields."),
        )
    }
    pub fn too_many_indexes(table_name: &TableName, num_indexes: usize) -> ErrorMetadata {
        ErrorMetadata::bad_request(
            "TooManyIndexes",
            format!("Table \"{table_name}\" cannot have more than {num_indexes} indexes."),
        )
    }
    pub fn too_many_search_indexes(table_name: &TableName, num_indexes: usize) -> ErrorMetadata {
        ErrorMetadata::bad_request(
            "TooManySearchIndexes",
            format!("Table \"{table_name}\" cannot have more than {num_indexes} search indexes."),
        )
    }
    pub fn too_many_vector_indexes(table_name: &TableName, num_indexes: usize) -> ErrorMetadata {
        ErrorMetadata::bad_request(
            "TooManyVectorIndexes",
            format!("Table \"{table_name}\" cannot have more than {num_indexes} vector indexes."),
        )
    }
    pub fn too_many_name_components(index_name: &str) -> ErrorMetadata {
        ErrorMetadata::bad_request(
            "IndexTooManyNameComponents",
            format!("Too many components in index name {index_name}"),
        )
    }

    // TODO move elsewhere (near table names) - it's not indexing related
    pub fn table_name_reserved(table_name: &TableName) -> ErrorMetadata {
        ErrorMetadata::bad_request(
            "TableNameReserved",
            format!("{table_name} is a reserved table name."),
        )
    }
    pub fn too_many_total_user_indexes(num_total_indexes: usize) -> ErrorMetadata {
        ErrorMetadata::bad_request(
            "TooManyTotalIndexes",
            format!("Number of total indexes cannot exceed {num_total_indexes}."),
        )
    }

    // TODO move elsewhere - it's not indexing related
    pub fn too_many_tables(num_tables: usize) -> ErrorMetadata {
        ErrorMetadata::bad_request(
            "TooManyTables",
            format!("Number of tables cannot exceed {num_tables}."),
        )
    }
}

// --------------------------------------------------------------------------------

////////////////////////////////////////////////////////////////////////////////

/// Configuration that depends on the type of index.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum IndexConfig {
    /// Standard database index.
    Database {
        developer_config: DeveloperDatabaseIndexConfig,

        /// Whether the index is fully backfilled or not on disk.
        on_disk_state: DatabaseIndexState,
    },

    /// Full text search index.
    Search {
        developer_config: DeveloperSearchIndexConfig,

        /// Whether the index is fully backfilled or not on disk.
        on_disk_state: SearchIndexState,
    },

    Vector {
        developer_config: DeveloperVectorIndexConfig,
        on_disk_state: VectorIndexState,
    },
}

impl IndexConfig {
    pub fn is_enabled(&self) -> bool {
        match self {
            IndexConfig::Database { on_disk_state, .. } => {
                matches!(on_disk_state, DatabaseIndexState::Enabled)
            },
            IndexConfig::Search { on_disk_state, .. } => {
                matches!(on_disk_state, SearchIndexState::SnapshottedAt(_))
            },
            IndexConfig::Vector { on_disk_state, .. } => {
                matches!(on_disk_state, VectorIndexState::SnapshottedAt(_))
            },
        }
    }

    pub fn is_backfilling(&self) -> bool {
        match self {
            IndexConfig::Database { on_disk_state, .. } => {
                matches!(on_disk_state, DatabaseIndexState::Backfilling(_))
            },
            IndexConfig::Search { on_disk_state, .. } => {
                matches!(on_disk_state, SearchIndexState::Backfilling)
            },
            IndexConfig::Vector { on_disk_state, .. } => {
                matches!(on_disk_state, VectorIndexState::Backfilling(_))
            },
        }
    }

    pub fn same_config(&self, config: &IndexConfig) -> bool {
        match (self, config) {
            (
                IndexConfig::Database {
                    developer_config, ..
                },
                IndexConfig::Database {
                    developer_config: config_to_compare,
                    ..
                },
            ) => developer_config == config_to_compare,
            (
                IndexConfig::Search {
                    developer_config, ..
                },
                IndexConfig::Search {
                    developer_config: config_to_compare,
                    ..
                },
            ) => developer_config == config_to_compare,
            (
                IndexConfig::Vector {
                    developer_config, ..
                },
                IndexConfig::Vector {
                    developer_config: config_to_compare,
                    ..
                },
            ) => developer_config == config_to_compare,
            (..) => false,
        }
    }

    /// Returns the estimated size of the index in bytes in a manner suitable
    /// for usage and pricing.
    ///
    /// The estimate here may not accurately reflect the actual number of
    /// stored bytes and may not be appropriate for estimate resource usage. For
    /// example, small dimension vector indexes may have 20% overhead from
    /// HNSW indexes that won't be reflected here, but would require
    /// additional RAM or disk space to serve.
    ///
    /// This is only implemented for vector indexes for now. Calling this method
    /// on other index types will panic.
    pub fn estimate_pricing_size_bytes(&self) -> anyhow::Result<u64> {
        match self {
            IndexConfig::Database { .. } | IndexConfig::Search { .. } => {
                // TODO(sam): We should support this for all index types in the future. Right
                // now search indexes are free and we estimate the size of
                // database indexes. Both of those could instead track usage in their metadata,
                // similar to vector indexes.
                anyhow::bail!("Only supported for vector indexes!")
            },
            IndexConfig::Vector {
                on_disk_state,
                developer_config,
            } => match on_disk_state {
                VectorIndexState::Backfilling(_) | VectorIndexState::Backfilled(_) => Ok(0),
                VectorIndexState::SnapshottedAt(snapshot) => match &snapshot.data {
                    VectorIndexSnapshotData::Unknown(_) => Ok(0),
                    VectorIndexSnapshotData::MultiSegment(segments) => segments
                        .iter()
                        .map(|segment| segment.non_deleted_size_bytes(developer_config.dimensions))
                        .sum::<anyhow::Result<_>>(),
                },
            },
        }
    }
}

impl TryFrom<IndexConfig> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(index_config: IndexConfig) -> anyhow::Result<Self> {
        match index_config {
            IndexConfig::Database {
                developer_config,
                on_disk_state,
            } => {
                let object: ConvexObject = obj!(
                    "type" => "database",
                    "onDiskState" => ConvexValue::Object(on_disk_state.try_into()?),
                )?;
                // TODO: Using merge here is very sketchy. Seems like DeveloperIndexConfig also
                // adds "type" but it happens to match the value from above.
                object.shallow_merge(ConvexObject::try_from(DeveloperIndexConfig::Database(
                    developer_config,
                ))?)
            },
            IndexConfig::Search {
                developer_config,
                on_disk_state,
            } => {
                let object = obj!(
                    "type" => "search",
                    "onDiskState" => ConvexValue::Object(on_disk_state.try_into()?)
                )?;
                // TODO: Using merge here is very sketchy. Seems like DeveloperIndexConfig also
                // adds "type" but it happens to match the value from above.
                object.shallow_merge(ConvexObject::try_from(DeveloperIndexConfig::Search(
                    developer_config,
                ))?)
            },
            IndexConfig::Vector {
                developer_config,
                on_disk_state,
            } => {
                let object = obj!(
                    "type" => "vector",
                    "onDiskState" => ConvexValue::Object(on_disk_state.try_into()?)
                )?;
                // TODO: Using merge here is very sketchy. Seems like DeveloperIndexConfig also
                // adds "type" but it happens to match the value from above.
                object.shallow_merge(ConvexObject::try_from(DeveloperIndexConfig::Vector(
                    developer_config,
                ))?)
            },
        }
    }
}

impl TryFrom<ConvexObject> for IndexConfig {
    type Error = anyhow::Error;

    fn try_from(object: ConvexObject) -> anyhow::Result<Self> {
        let mut object_fields: BTreeMap<_, _> = object.into();
        let config_type = match object_fields.remove("type") {
            Some(ConvexValue::String(s)) => s,
            _ => anyhow::bail!("Missing `type` field for IndexConfig: {:?}", object_fields),
        };

        Ok(match config_type.to_string().as_str() {
            "database" => {
                let fields = match object_fields.remove("fields") {
                    Some(v) => Vec::<FieldPath>::try_from(v)?.try_into()?,
                    _ => anyhow::bail!(
                        "Missing `fields` field for IndexConfig: {:?}",
                        object_fields
                    ),
                };
                let on_disk_state = match object_fields.remove("onDiskState") {
                    Some(ConvexValue::Object(o)) => o.try_into()?,
                    _ => anyhow::bail!(
                        "Invalid or missing`onDiskState` field for IndexConfig: {:?}",
                        object_fields
                    ),
                };
                IndexConfig::Database {
                    developer_config: DeveloperDatabaseIndexConfig { fields },
                    on_disk_state,
                }
            },
            "search" => {
                let search_field = match object_fields.remove("searchField") {
                    Some(v) => v.try_into()?,
                    _ => anyhow::bail!(
                        "Missing `searchField` field for IndexConfig: {:?}",
                        object_fields
                    ),
                };
                let filter_fields = match object_fields.remove("filterFields") {
                    Some(ConvexValue::Array(arr)) => arr
                        .into_iter()
                        .map(FieldPath::try_from)
                        .collect::<anyhow::Result<BTreeSet<_>>>()?,
                    _ => anyhow::bail!(
                        "Missing `filterFields` field for IndexConfig: {:?}",
                        object_fields
                    ),
                };
                let on_disk_state = match object_fields.remove("onDiskState") {
                    Some(ConvexValue::Object(o)) => o.try_into()?,
                    _ => anyhow::bail!(
                        "Invalid or missing`onDiskState` field for IndexConfig: {:?}",
                        object_fields
                    ),
                };
                IndexConfig::Search {
                    developer_config: DeveloperSearchIndexConfig {
                        search_field,
                        filter_fields,
                    },
                    on_disk_state,
                }
            },
            "vector" => {
                let dimensions = match object_fields.remove("dimensions") {
                    Some(ConvexValue::Int64(dimensions)) => {
                        VectorDimensions::try_from(u32::try_from(dimensions)?)?
                    },
                    // Support legacy alpha users with the old dimension field.
                    None => match object_fields.remove("dimension") {
                        Some(ConvexValue::Int64(dimension)) => {
                            VectorDimensions::try_from(u32::try_from(dimension)?)?
                        },
                        _ => anyhow::bail!(
                            "Invalid or missing `dimension` field for IndexConfig: {:?}",
                            object_fields
                        ),
                    },
                    _ => anyhow::bail!(
                        "Invalid or missing `dimensions` field for IndexConfig: {:?}",
                        object_fields
                    ),
                };
                let vector_field = match object_fields.remove("vectorField") {
                    Some(v) => v.try_into()?,
                    _ => anyhow::bail!(
                        "Missing `vectorField` field for IndexConfig: {:?}",
                        object_fields
                    ),
                };
                let filter_fields = match object_fields.remove("filterFields") {
                    Some(ConvexValue::Array(arr)) => arr
                        .into_iter()
                        .map(FieldPath::try_from)
                        .collect::<anyhow::Result<BTreeSet<_>>>()?,
                    _ => anyhow::bail!(
                        "Missing `filterFields` field for IndexConfig: {:?}",
                        object_fields
                    ),
                };
                let on_disk_state = match object_fields.remove("onDiskState") {
                    Some(ConvexValue::Object(o)) => o.try_into()?,
                    _ => anyhow::bail!(
                        "Invalid or missing`onDiskState` field for IndexConfig: {:?}",
                        object_fields
                    ),
                };
                IndexConfig::Vector {
                    developer_config: DeveloperVectorIndexConfig {
                        dimensions,
                        vector_field,
                        filter_fields,
                    },
                    on_disk_state,
                }
            },
            _ => anyhow::bail!("Invalid `type` field for IndexConfig: {:?}", object_fields),
        })
    }
}

// Index config that's specified by the developer
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum DeveloperIndexConfig {
    /// Standard database index.
    Database(DeveloperDatabaseIndexConfig),

    /// Full text search index.
    Search(DeveloperSearchIndexConfig),

    Vector(DeveloperVectorIndexConfig),
}

impl From<IndexConfig> for DeveloperIndexConfig {
    fn from(value: IndexConfig) -> Self {
        match value {
            IndexConfig::Database {
                developer_config, ..
            } => DeveloperIndexConfig::Database(developer_config),
            IndexConfig::Search {
                developer_config, ..
            } => DeveloperIndexConfig::Search(developer_config),
            IndexConfig::Vector {
                developer_config, ..
            } => DeveloperIndexConfig::Vector(developer_config),
        }
    }
}

impl TryFrom<DeveloperIndexConfig> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(index_config: DeveloperIndexConfig) -> anyhow::Result<Self> {
        match index_config {
            DeveloperIndexConfig::Database(config) => {
                obj!(
                    "type" => "database",
                    "fields" => config.fields,
                )
            },
            DeveloperIndexConfig::Search(config) => {
                let filter_fields = config
                    .filter_fields
                    .into_iter()
                    .map(ConvexValue::try_from)
                    .collect::<anyhow::Result<Vec<_>>>()?;
                obj!(
                    "type" => "search",
                    "searchField" => config.search_field,
                    "filterFields" => filter_fields,
                )
            },
            DeveloperIndexConfig::Vector(config) => {
                let filter_fields = config
                    .filter_fields
                    .into_iter()
                    .map(ConvexValue::try_from)
                    .collect::<anyhow::Result<Vec<_>>>()?;
                obj!(
                    "type" => "vector",
                    "dimensions" => (u32::from(config.dimensions) as i64),
                    "vectorField" => config.vector_field,
                    "filterFields" => filter_fields,
                )
            },
        }
    }
}

impl TryFrom<ConvexObject> for DeveloperIndexConfig {
    type Error = anyhow::Error;

    fn try_from(obj: ConvexObject) -> anyhow::Result<Self> {
        let mut fields: BTreeMap<FieldName, ConvexValue> = obj.into();
        let type_string = match fields.remove("type") {
            Some(ConvexValue::String(s)) => s.to_string(),
            Some(_) => anyhow::bail!("Invalid value for `type`"),
            None => anyhow::bail!("Missing field `type`"),
        };
        if type_string == "database" {
            let indexed_fields = match fields.remove("fields") {
                Some(fields_value) => database_index::IndexedFields::try_from(fields_value)?,
                None => anyhow::bail!("Missing field `fields`"),
            };

            Ok(DeveloperIndexConfig::Database(
                DeveloperDatabaseIndexConfig {
                    fields: indexed_fields,
                },
            ))
        } else if type_string == "search" {
            let filter_fields = match fields.remove("filterFields") {
                Some(ConvexValue::Array(filter_fields_arr)) => filter_fields_arr
                    .into_iter()
                    .map(FieldPath::try_from)
                    .collect::<anyhow::Result<BTreeSet<_>>>()?,
                Some(_) => anyhow::bail!("Invalid value for filterFields"),
                None => anyhow::bail!("Missing field filterFields"),
            };
            let search_field = match fields.remove("searchField") {
                Some(val) => FieldPath::try_from(val)?,
                None => anyhow::bail!("Missing field searchField"),
            };
            return Ok(DeveloperIndexConfig::Search(DeveloperSearchIndexConfig {
                search_field,
                filter_fields,
            }));
        } else if type_string == "vector" {
            let dimensions = match fields.remove("dimensions") {
                Some(ConvexValue::Int64(dimensions)) => {
                    VectorDimensions::try_from(u32::try_from(dimensions)?)?
                },
                // Support legacy alpha users with the old dimension field.
                None => match fields.remove("dimension") {
                    Some(ConvexValue::Int64(dimension)) => {
                        VectorDimensions::try_from(u32::try_from(dimension)?)?
                    },
                    _ => anyhow::bail!("Invalid value for dimension"),
                },
                _ => anyhow::bail!("Invalid value for dimensions"),
            };
            let filter_fields = match fields.remove("filterFields") {
                Some(ConvexValue::Array(filter_fields)) => filter_fields
                    .into_iter()
                    .map(FieldPath::try_from)
                    .collect::<anyhow::Result<BTreeSet<_>>>()?,
                _ => anyhow::bail!("Invalid value for filterFields"),
            };
            let vector_field = match fields.remove("vectorField") {
                Some(val) => FieldPath::try_from(val)?,
                None => anyhow::bail!("Missing field vectorField"),
            };
            return Ok(DeveloperIndexConfig::Vector(DeveloperVectorIndexConfig {
                dimensions,
                vector_field,
                filter_fields,
            }));
        } else {
            anyhow::bail!("Unknown type {type_string}")
        }
    }
}

impl TryFrom<ConvexValue> for DeveloperIndexConfig {
    type Error = anyhow::Error;

    fn try_from(value: ConvexValue) -> Result<Self, Self::Error> {
        if let ConvexValue::Object(obj) = value {
            obj.try_into()
        } else {
            anyhow::bail!("Invalid value for DeveloperIndexConfig")
        }
    }
}

impl TryFrom<DeveloperIndexConfig> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(value: DeveloperIndexConfig) -> Result<Self, Self::Error> {
        Ok(ConvexObject::try_from(value)?.into())
    }
}

pub type ResolvedIndexMetadata = IndexMetadata<TableIdAndTableNumber>;
pub type TabletIndexMetadata = IndexMetadata<TableId>;
pub type DeveloperIndexMetadata = IndexMetadata<TableName>;

impl From<ResolvedIndexMetadata> for IndexMetadata<TableId> {
    fn from(value: ResolvedIndexMetadata) -> Self {
        Self {
            name: value.name.into(),
            config: value.config,
        }
    }
}

impl ResolvedIndexMetadata {
    pub fn from_document(
        f: impl Fn(TableId) -> anyhow::Result<TableIdAndTableNumber>,
        document: ResolvedDocument,
    ) -> anyhow::Result<ParsedDocument<Self>> {
        let index_metadata_: ParsedDocument<TabletIndexMetadata> = document.try_into()?;
        let index_metadata: ParsedDocument<Self> = index_metadata_.map(|d| d.map_table(&f))?;
        Ok(index_metadata)
    }
}

impl TabletIndexMetadata {
    pub fn from_document(document: ResolvedDocument) -> anyhow::Result<ParsedDocument<Self>> {
        document.try_into()
    }
}
/// In-memory representation of an index's metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct IndexMetadata<T: TableIdentifier> {
    /// Unique name for the index.
    pub name: GenericIndexName<T>,

    /// Configuration that depends on the type of index.
    pub config: IndexConfig,
}

impl<T: TableIdentifier> IndexMetadata<T> {
    pub fn new_backfilling(
        name: GenericIndexName<T>,
        fields: database_index::IndexedFields,
    ) -> Self {
        Self {
            name,
            config: IndexConfig::Database {
                developer_config: DeveloperDatabaseIndexConfig { fields },
                on_disk_state: DatabaseIndexState::Backfilling(
                    database_index::DatabaseIndexBackfillState {},
                ),
            },
        }
    }

    pub fn new_backfilling_search_index(
        name: GenericIndexName<T>,
        search_field: FieldPath,
        filter_fields: BTreeSet<FieldPath>,
    ) -> Self {
        Self::new_search_index(
            name,
            DeveloperSearchIndexConfig {
                search_field,
                filter_fields,
            },
            SearchIndexState::Backfilling,
        )
    }

    pub fn new_backfilling_vector_index(
        name: GenericIndexName<T>,
        vector_field: FieldPath,
        dimensions: VectorDimensions,
        filter_fields: BTreeSet<FieldPath>,
    ) -> Self {
        Self {
            name,
            config: IndexConfig::Vector {
                developer_config: DeveloperVectorIndexConfig {
                    dimensions,
                    vector_field,
                    filter_fields,
                },
                on_disk_state: VectorIndexState::Backfilling(VectorIndexBackfillState {
                    segments: vec![],
                    cursor: None,
                    backfill_snapshot_ts: None,
                }),
            },
        }
    }

    pub fn new_search_index(
        name: GenericIndexName<T>,
        developer_config: DeveloperSearchIndexConfig,
        on_disk_state: SearchIndexState,
    ) -> Self {
        Self {
            name,
            config: IndexConfig::Search {
                developer_config,
                on_disk_state,
            },
        }
    }

    pub fn new_enabled(name: GenericIndexName<T>, fields: database_index::IndexedFields) -> Self {
        Self {
            name,
            config: IndexConfig::Database {
                developer_config: DeveloperDatabaseIndexConfig { fields },
                on_disk_state: DatabaseIndexState::Enabled,
            },
        }
    }

    pub fn is_database_index(&self) -> bool {
        matches!(self.config, IndexConfig::Database { .. })
    }

    pub fn is_search_index(&self) -> bool {
        matches!(self.config, IndexConfig::Search { .. })
    }

    pub fn is_vector_index(&self) -> bool {
        matches!(self.config, IndexConfig::Vector { .. })
    }

    pub fn map_table<U: TableIdentifier>(
        self,
        f: &impl Fn(T) -> anyhow::Result<U>,
    ) -> anyhow::Result<IndexMetadata<U>> {
        Ok(IndexMetadata {
            name: self.name.map_table(f)?,
            config: self.config,
        })
    }
}

pub fn index_metadata_serialize_table_id(table_id: &TableId) -> anyhow::Result<ConvexValue> {
    ConvexValue::try_from(table_id.to_string())
}

impl TryFrom<TabletIndexMetadata> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(m: TabletIndexMetadata) -> anyhow::Result<Self> {
        let name = m.name;
        obj!(
            // New format: write table_id(v5) + descriptor.
            *TABLE_ID_FIELD_NAME => index_metadata_serialize_table_id(name.table())?,
            "descriptor" => name.descriptor().to_string(),
            "config" => ConvexObject::try_from(m.config)?
        )
    }
}

impl TryFrom<ConvexObject> for IndexMetadata<TableId> {
    type Error = anyhow::Error;

    fn try_from(o: ConvexObject) -> Result<Self, Self::Error> {
        let mut object_fields: BTreeMap<_, _> = o.into();
        let table_id: TableId = match object_fields.remove("table_id") {
            Some(ConvexValue::String(s)) => s.parse()?,
            _ => anyhow::bail!(
                "Missing or invalid `table_id` field for IndexMetadata: {:?}",
                object_fields
            ),
        };
        let descriptor: IndexDescriptor = match object_fields.remove("descriptor") {
            Some(ConvexValue::String(d)) => d.parse()?,
            _ => anyhow::bail!(
                "Missing or invalid `table_id` field for IndexMetadata: {:?}",
                object_fields
            ),
        };
        let name = if descriptor.is_reserved() {
            GenericIndexName::new_reserved(table_id, descriptor)
        } else {
            GenericIndexName::new(table_id, descriptor)
        }?;
        let config = match object_fields.remove("config") {
            Some(ConvexValue::Object(config)) => IndexConfig::try_from(config)?,
            _ => anyhow::bail!(
                "Missing or invalid `config` field for IndexMetadata: {:?}",
                object_fields
            ),
        };

        Ok(Self { name, config })
    }
}

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use proptest::prelude::*;
    use value::ConvexObject;

    use super::*;
    use crate::testing::assert_roundtrips;

    proptest! {
        #![proptest_config(ProptestConfig { cases: 64 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]
        #[test]
        fn test_indexed_config_roundtrips(config in any::<IndexConfig>()) {
            assert_roundtrips::<IndexConfig, ConvexObject>(config);
        }

        #[test]
        fn test_developer_index_config_roundtrips(config in any::<DeveloperIndexConfig>()) {
            assert_roundtrips::<DeveloperIndexConfig, ConvexObject>(config);
        }
    }
}
