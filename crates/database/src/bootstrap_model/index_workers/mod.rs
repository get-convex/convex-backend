use std::{
    cmp::max,
    str::FromStr,
    sync::LazyLock,
};

use common::{
    document::{
        ParseDocument,
        ParsedDocument,
    },
    query::{
        IndexRange,
        IndexRangeExpression,
        Order,
        Query,
    },
    runtime::Runtime,
    types::IndexId,
};
use serde::{
    Deserialize,
    Serialize,
};
use sync_types::Timestamp;
use value::{
    codegen_convex_serialization,
    ConvexValue,
    FieldPath,
    InternalId,
    TableName,
    TableNamespace,
};

use crate::{
    system_tables::{
        SystemIndex,
        SystemTable,
    },
    ResolvedQuery,
    SystemMetadataModel,
    Transaction,
};

pub struct IndexWorkerMetadataModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> IndexWorkerMetadataModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    pub async fn get_fast_forward_ts(
        &mut self,
        snapshot_ts: Timestamp,
        index_id: IndexId,
    ) -> anyhow::Result<Timestamp> {
        let metadata = self.get_metadata(index_id).await?;

        let fast_forward_ts = metadata
            .map(|meta| *meta.into_value().index_metadata.mut_fast_forward_ts())
            .unwrap_or_default();
        Ok(max(snapshot_ts, fast_forward_ts))
    }

    pub async fn get_or_create_vector_search(
        &mut self,
        id: IndexId,
    ) -> anyhow::Result<ParsedDocument<IndexWorkerMetadataRecord>> {
        self.get_or_create_metadata(
            id,
            IndexWorkerMetadata::VectorSearch(IndexWorkerBatchMetadata {
                fast_forward_ts: Timestamp::default(),
            }),
        )
        .await
    }

    pub async fn get_or_create_text_search(
        &mut self,
        id: IndexId,
    ) -> anyhow::Result<ParsedDocument<IndexWorkerMetadataRecord>> {
        self.get_or_create_metadata(
            id,
            IndexWorkerMetadata::TextSearch(IndexWorkerBatchMetadata {
                fast_forward_ts: Timestamp::default(),
            }),
        )
        .await
    }

    pub async fn get_metadata(
        &mut self,
        id: IndexId,
    ) -> anyhow::Result<Option<ParsedDocument<IndexWorkerMetadataRecord>>> {
        let range = vec![IndexRangeExpression::Eq(
            INDEX_DOC_ID_FIELD.clone(),
            ConvexValue::String(id.to_string().try_into()?).into(),
        )];
        let query = Query::index_range(IndexRange {
            index_name: INDEX_DOC_ID_INDEX.name(),
            range,
            order: Order::Asc,
        });
        let mut query_stream = ResolvedQuery::new(self.tx, TableNamespace::Global, query)?;
        let result = query_stream.next(self.tx, None).await?;
        result
            .map(ParseDocument::<IndexWorkerMetadataRecord>::parse)
            .transpose()
    }

    async fn get_or_create_metadata(
        &mut self,
        id: IndexId,
        default: IndexWorkerMetadata,
    ) -> anyhow::Result<ParsedDocument<IndexWorkerMetadataRecord>> {
        let existing_doc = self.get_metadata(id).await?;
        if let Some(doc) = existing_doc {
            return Ok(doc);
        }
        self.create_metadata(IndexWorkerMetadataRecord {
            index_id: id,
            index_metadata: default,
        })
        .await
    }

    async fn create_metadata(
        &mut self,
        metadata: IndexWorkerMetadataRecord,
    ) -> anyhow::Result<ParsedDocument<IndexWorkerMetadataRecord>> {
        let id = SystemMetadataModel::new_global(self.tx)
            .insert(&INDEX_WORKER_METADATA_TABLE, metadata.try_into()?)
            .await?;
        ParseDocument::parse(self.tx.get(id).await?.unwrap())
    }
}

pub static INDEX_WORKER_METADATA_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_index_worker_metadata"
        .parse()
        .expect("_index_worker_metadata is an invalid table name")
});

static INDEX_DOC_ID_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "index_id".parse().expect("Invalid built-in field"));

pub static INDEX_DOC_ID_INDEX: LazyLock<SystemIndex<IndexWorkerMetadataTable>> =
    LazyLock::new(|| SystemIndex::new("by_index_doc_id", [&INDEX_DOC_ID_FIELD]).unwrap());

pub struct IndexWorkerMetadataTable;
impl SystemTable for IndexWorkerMetadataTable {
    type Metadata = IndexWorkerMetadataRecord;

    fn table_name() -> &'static TableName {
        &INDEX_WORKER_METADATA_TABLE
    }

    fn indexes() -> Vec<SystemIndex<Self>> {
        vec![INDEX_DOC_ID_INDEX.clone()]
    }
}

/// Metadata used by index workers to build indexes more efficiently that does
/// not impact the state of the index or the data returned by the index.
///
/// Metadata that impacts how the index is queried belongs in the actual index
/// metadata. This must be used only within the index worker as an
/// implementation detail of how the index is built.
#[derive(Debug)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(proptest_derive::Arbitrary, Clone, PartialEq)
)]
pub struct IndexWorkerMetadataRecord {
    index_id: InternalId,
    pub index_metadata: IndexWorkerMetadata,
}

#[derive(Serialize, Deserialize)]
pub struct SerializedIndexWorkerMetadataRecord {
    pub index_id: String,
    pub index_metadata: SerializedIndexWorkerMetadata,
}

codegen_convex_serialization!(
    IndexWorkerMetadataRecord,
    SerializedIndexWorkerMetadataRecord
);

impl From<IndexWorkerMetadataRecord> for SerializedIndexWorkerMetadataRecord {
    fn from(value: IndexWorkerMetadataRecord) -> Self {
        SerializedIndexWorkerMetadataRecord {
            index_id: value.index_id.to_string(),
            index_metadata: value.index_metadata.into(),
        }
    }
}

impl TryFrom<SerializedIndexWorkerMetadataRecord> for IndexWorkerMetadataRecord {
    type Error = anyhow::Error;

    fn try_from(value: SerializedIndexWorkerMetadataRecord) -> Result<Self, Self::Error> {
        let index_id = InternalId::from_str(&value.index_id)?;
        let index_metadata = IndexWorkerMetadata::try_from(value.index_metadata)?;
        Ok(IndexWorkerMetadataRecord {
            index_id,
            index_metadata,
        })
    }
}

#[derive(Debug)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(proptest_derive::Arbitrary, Clone, PartialEq)
)]
pub enum IndexWorkerMetadata {
    TextSearch(IndexWorkerBatchMetadata),
    VectorSearch(IndexWorkerBatchMetadata),
}

impl IndexWorkerMetadata {
    pub fn mut_fast_forward_ts(&mut self) -> &mut Timestamp {
        &mut match self {
            IndexWorkerMetadata::TextSearch(ref mut meta) => meta,
            IndexWorkerMetadata::VectorSearch(ref mut meta) => meta,
        }
        .fast_forward_ts
    }
}

#[derive(Serialize, Deserialize)]
pub struct SerializedIndexWorkerMetadata {
    pub metadata_type: String,
    pub metadata: SerializedIndexWorkerBatchMetadata,
}

impl From<IndexWorkerMetadata> for SerializedIndexWorkerMetadata {
    fn from(value: IndexWorkerMetadata) -> Self {
        let (metadata_type, metadata) = match value {
            IndexWorkerMetadata::TextSearch(metadata) => ("text_search", metadata),
            IndexWorkerMetadata::VectorSearch(metadata) => ("vector_search", metadata),
        };
        SerializedIndexWorkerMetadata {
            metadata_type: metadata_type.to_string(),
            metadata: metadata.into(),
        }
    }
}

impl TryFrom<SerializedIndexWorkerMetadata> for IndexWorkerMetadata {
    type Error = anyhow::Error;

    fn try_from(value: SerializedIndexWorkerMetadata) -> Result<Self, Self::Error> {
        let metadata = IndexWorkerBatchMetadata::try_from(value.metadata)?;
        Ok(match value.metadata_type.as_str() {
            "text_search" => IndexWorkerMetadata::TextSearch(metadata),
            "vector_search" => IndexWorkerMetadata::VectorSearch(metadata),
            metadata_type => anyhow::bail!(
                "Invalid `metadata_type` field value for IndexWorkerMetadata: {metadata_type}",
            ),
        })
    }
}

/// Metadata for index workers that process documents in the background in
/// batches on an ongoing basis.
///
/// For now this is vector and text search.
#[derive(Debug)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(proptest_derive::Arbitrary, Clone, PartialEq)
)]
pub struct IndexWorkerBatchMetadata {
    fast_forward_ts: Timestamp,
}

#[derive(Serialize, Deserialize)]
pub struct SerializedIndexWorkerBatchMetadata {
    pub fast_forward_ts: i64,
}

impl From<IndexWorkerBatchMetadata> for SerializedIndexWorkerBatchMetadata {
    fn from(value: IndexWorkerBatchMetadata) -> Self {
        SerializedIndexWorkerBatchMetadata {
            fast_forward_ts: value.fast_forward_ts.into(),
        }
    }
}

impl TryFrom<SerializedIndexWorkerBatchMetadata> for IndexWorkerBatchMetadata {
    type Error = anyhow::Error;

    fn try_from(value: SerializedIndexWorkerBatchMetadata) -> Result<Self, Self::Error> {
        let fast_forward_ts = Timestamp::try_from(value.fast_forward_ts)?;
        Ok(IndexWorkerBatchMetadata { fast_forward_ts })
    }
}

#[cfg(test)]
mod tests {
    use sync_types::Timestamp;
    use value::{
        assert_obj,
        InternalId,
    };

    use crate::bootstrap_model::index_workers::{
        IndexWorkerBatchMetadata,
        IndexWorkerMetadata,
        IndexWorkerMetadataRecord,
    };

    #[test]
    fn test_frozen_obj() {
        assert_eq!(
            IndexWorkerMetadataRecord::try_from(assert_obj! {
                "index_id" => "BJbr11wH980UmedO1cm7Aw",
                "index_metadata" => assert_obj! {
                    "metadata" => assert_obj! { "fast_forward_ts" => 6067670556026735204i64 },
                    "metadata_type" => "text_search",
                },
            })
            .unwrap(),
            IndexWorkerMetadataRecord {
                index_id: InternalId::from_developer_str("BJbr11wH980UmedO1cm7Aw").unwrap(),
                index_metadata: IndexWorkerMetadata::TextSearch(IndexWorkerBatchMetadata {
                    fast_forward_ts: Timestamp::try_from(6067670556026735204i64).unwrap()
                })
            }
        );
    }
}
