use std::collections::BTreeSet;

use serde::{
    Deserialize,
    Serialize,
};
use sync_types::Timestamp;
use value::{
    codegen_convex_serialization,
    ConvexValue,
    FieldPath,
    TableIdentifier,
    TableName,
    TabletId,
    TabletIdAndTableNumber,
};

use super::{
    database_index::{
        DatabaseIndexBackfillState,
        DatabaseIndexState,
        DeveloperDatabaseIndexConfig,
        IndexedFields,
    },
    index_config::SerializedIndexConfig,
    vector_index::{
        DeveloperVectorIndexConfig,
        VectorDimensions,
        VectorIndexBackfillState,
        VectorIndexState,
    },
    IndexConfig,
};
use crate::{
    bootstrap_model::index::text_index::{
        DeveloperSearchIndexConfig,
        TextIndexBackfillState,
        TextIndexState,
    },
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    types::{
        GenericIndexName,
        IndexDescriptor,
    },
};

pub type ResolvedIndexMetadata = IndexMetadata<TabletIdAndTableNumber>;
pub type TabletIndexMetadata = IndexMetadata<TabletId>;
pub type DeveloperIndexMetadata = IndexMetadata<TableName>;

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
        index_created_lower_bound: Timestamp,
        name: GenericIndexName<T>,
        fields: IndexedFields,
    ) -> Self {
        Self {
            name,
            config: IndexConfig::Database {
                developer_config: DeveloperDatabaseIndexConfig { fields },
                on_disk_state: DatabaseIndexState::Backfilling(DatabaseIndexBackfillState {
                    index_created_lower_bound,
                    retention_started: false,
                }),
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
            TextIndexState::Backfilling(TextIndexBackfillState::new()),
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
        on_disk_state: TextIndexState,
    ) -> Self {
        Self {
            name,
            config: IndexConfig::Search {
                developer_config,
                on_disk_state,
            },
        }
    }

    pub fn new_enabled(name: GenericIndexName<T>, fields: IndexedFields) -> Self {
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

impl From<ResolvedIndexMetadata> for TabletIndexMetadata {
    fn from(value: ResolvedIndexMetadata) -> Self {
        Self {
            name: value.name.into(),
            config: value.config,
        }
    }
}

impl ResolvedIndexMetadata {
    pub fn from_document(
        f: impl Fn(TabletId) -> anyhow::Result<TabletIdAndTableNumber>,
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

pub fn index_metadata_serialize_tablet_id(tablet_id: &TabletId) -> anyhow::Result<ConvexValue> {
    ConvexValue::try_from(tablet_id.to_string())
}

#[derive(Serialize, Deserialize)]
struct SerializedTabletIndexMetadata {
    table_id: String,
    descriptor: String,
    config: SerializedIndexConfig,
}

impl TryFrom<TabletIndexMetadata> for SerializedTabletIndexMetadata {
    type Error = anyhow::Error;

    fn try_from(m: TabletIndexMetadata) -> anyhow::Result<Self> {
        Ok(Self {
            // New format: write table_id(v5) + descriptor.
            table_id: m.name.table().to_string(),
            descriptor: m.name.descriptor().to_string(),
            config: m.config.try_into()?,
        })
    }
}

impl TryFrom<SerializedTabletIndexMetadata> for TabletIndexMetadata {
    type Error = anyhow::Error;

    fn try_from(s: SerializedTabletIndexMetadata) -> anyhow::Result<Self> {
        let table_id: TabletId = s.table_id.parse()?;
        let descriptor: IndexDescriptor = s.descriptor.parse()?;
        let name = if descriptor.is_reserved() {
            GenericIndexName::new_reserved(table_id, descriptor)
        } else {
            GenericIndexName::new(table_id, descriptor)
        }?;
        let config = IndexConfig::try_from(s.config)?;
        Ok(Self { name, config })
    }
}

codegen_convex_serialization!(TabletIndexMetadata, SerializedTabletIndexMetadata);
