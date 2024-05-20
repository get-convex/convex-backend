use std::{
    cmp::max,
    collections::BTreeMap,
    str::FromStr,
    sync::LazyLock,
};

use common::{
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    query::{
        IndexRange,
        IndexRangeExpression,
        Order,
        Query,
    },
    runtime::Runtime,
    types::{
        IndexId,
        IndexName,
    },
};
use sync_types::Timestamp;
use value::{
    obj,
    ConvexObject,
    ConvexValue,
    FieldPath,
    InternalId,
    TableName,
    TableNamespace,
};

use crate::{
    defaults::{
        system_index,
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
            index_name: INDEX_DOC_ID_INDEX.clone(),
            range,
            order: Order::Asc,
        });
        let mut query_stream = ResolvedQuery::new(self.tx, TableNamespace::Global, query)?;
        let result = query_stream.next(self.tx, None).await?;
        result
            .map(ParsedDocument::<IndexWorkerMetadataRecord>::try_from)
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
        let id = SystemMetadataModel::new(self.tx)
            .insert(&INDEX_WORKER_METADATA_TABLE, metadata.try_into()?)
            .await?;
        ParsedDocument::try_from(self.tx.get(id).await?.unwrap())
    }
}

pub static INDEX_WORKER_METADATA_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_index_worker_metadata"
        .parse()
        .expect("_index_worker_metadata is an invalid table name")
});

static INDEX_DOC_ID_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "index_id".parse().expect("Invalid built-in field"));

pub static INDEX_DOC_ID_INDEX: LazyLock<IndexName> =
    LazyLock::new(|| system_index(&INDEX_WORKER_METADATA_TABLE, "by_index_doc_id"));

pub struct IndexWorkerMetadataTable;
impl SystemTable for IndexWorkerMetadataTable {
    fn table_name(&self) -> &'static TableName {
        &INDEX_WORKER_METADATA_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![SystemIndex {
            name: INDEX_DOC_ID_INDEX.clone(),
            fields: vec![INDEX_DOC_ID_FIELD.clone()].try_into().unwrap(),
        }]
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParsedDocument::<IndexWorkerMetadataRecord>::try_from(document).map(|_| ())
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

impl TryFrom<IndexWorkerMetadataRecord> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: IndexWorkerMetadataRecord) -> Result<Self, Self::Error> {
        obj!(
            "index_id" => ConvexValue::String(value.index_id.to_string().try_into()?),
            "index_metadata"=> ConvexValue::Object(value.index_metadata.try_into()?),
        )
    }
}

impl TryFrom<ConvexObject> for IndexWorkerMetadataRecord {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> Result<Self, Self::Error> {
        let mut fields: BTreeMap<_, _> = value.into();

        let index_id = match fields.remove("index_id") {
            Some(ConvexValue::String(index_id)) => {
                InternalId::from_str(index_id.to_string().as_str())?
            },
            _ => anyhow::bail!("Missing or invalid `index_id` field for IndexWorkerMetadataRecord"),
        };

        let index_metadata = match fields.remove("index_metadata") {
            Some(ConvexValue::Object(index_metadata)) => {
                IndexWorkerMetadata::try_from(index_metadata)?
            },
            _ => anyhow::bail!(
                "Missing or invalid `index_metadata` field for IndexWorkerMetadataRecord"
            ),
        };
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

impl TryFrom<IndexWorkerMetadata> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: IndexWorkerMetadata) -> Result<Self, Self::Error> {
        let (metadata_type, metadata) = match value {
            IndexWorkerMetadata::TextSearch(metadata) => ("text_search", metadata),
            IndexWorkerMetadata::VectorSearch(metadata) => ("vector_search", metadata),
        };
        obj!(
            "metadata_type" => ConvexValue::String(metadata_type.to_string().try_into()?),
            "metadata" => ConvexValue::Object(metadata.try_into()?),
        )
    }
}

impl TryFrom<ConvexObject> for IndexWorkerMetadata {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> Result<Self, Self::Error> {
        let mut fields: BTreeMap<_, _> = value.into();

        let metadata = match fields.remove("metadata") {
            Some(ConvexValue::Object(metadata)) => IndexWorkerBatchMetadata::try_from(metadata)?,
            _ => anyhow::bail!("Missing or invalid `metadata` field for IndexWorkerMetadata"),
        };

        let metadata_type = match fields.remove("metadata_type") {
            Some(ConvexValue::String(metadata_type)) => metadata_type,
            _ => anyhow::bail!("Missing or invalid `metadata_type` field for IndexWorkerMetadata"),
        };

        Ok(match metadata_type.to_string().as_str() {
            "text_search" => IndexWorkerMetadata::TextSearch(metadata),
            "vector_search" => IndexWorkerMetadata::VectorSearch(metadata),
            _ => anyhow::bail!(
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

impl TryFrom<IndexWorkerBatchMetadata> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: IndexWorkerBatchMetadata) -> Result<Self, Self::Error> {
        let ts = ConvexValue::Int64(value.fast_forward_ts.into());
        obj!(
            "fast_forward_ts" => ts,
        )
    }
}

impl TryFrom<ConvexObject> for IndexWorkerBatchMetadata {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> Result<Self, Self::Error> {
        let mut fields: BTreeMap<_, _> = value.into();
        let fast_forward_ts = match fields.remove("fast_forward_ts") {
            Some(ConvexValue::Int64(ts)) => Timestamp::try_from(ts)?,
            _ => {
                anyhow::bail!("Missing or invalid `fast_forward_ts` field for IndexWorkerMetadata")
            },
        };
        Ok(IndexWorkerBatchMetadata { fast_forward_ts })
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use value::ConvexObject;

    use crate::bootstrap_model::index_workers::IndexWorkerMetadataRecord;

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_metadata_roundtrip(v in any::<IndexWorkerMetadataRecord>()) {
            let roundtripped = IndexWorkerMetadataRecord::try_from(
                ConvexObject::try_from(v.clone()).unwrap()
            ).unwrap();
            assert_eq!(v, roundtripped);
        }

    }
}
