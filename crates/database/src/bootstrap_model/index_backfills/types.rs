use common::types::IndexId;
use serde::{
    Deserialize,
    Serialize,
};
use value::codegen_convex_serialization;

/// Metadata for tracking index backfill progress.
///
/// This structure stores the progress of an index backfill operation,
/// tracking how many documents and bytes have been processed out of the total.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct IndexBackfillMetadata {
    /// The ID of the index being backfilled. Should correspond to a document in
    /// the index table in `Backfilling` state.
    pub index_id: IndexId,
    /// Number of documents that have been indexed so far
    pub num_docs_indexed: u64,
    /// Number of bytes that have been indexed so far
    pub bytes_indexed: u64,
    /// Total number of documents in the table
    pub total_docs: u64,
    /// Total number of bytes in the table
    pub total_bytes: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedIndexBackfillMetadata {
    index_id: String,
    num_docs_indexed: i64,
    bytes_indexed: i64,
    total_docs: i64,
    total_bytes: i64,
}

impl From<IndexBackfillMetadata> for SerializedIndexBackfillMetadata {
    fn from(metadata: IndexBackfillMetadata) -> Self {
        SerializedIndexBackfillMetadata {
            index_id: metadata.index_id.to_string(),
            num_docs_indexed: metadata.num_docs_indexed as i64,
            bytes_indexed: metadata.bytes_indexed as i64,
            total_docs: metadata.total_docs as i64,
            total_bytes: metadata.total_bytes as i64,
        }
    }
}

impl TryFrom<SerializedIndexBackfillMetadata> for IndexBackfillMetadata {
    type Error = anyhow::Error;

    fn try_from(serialized: SerializedIndexBackfillMetadata) -> Result<Self, Self::Error> {
        Ok(Self {
            index_id: serialized.index_id.parse()?,
            num_docs_indexed: serialized.num_docs_indexed as u64,
            bytes_indexed: serialized.bytes_indexed as u64,
            total_docs: serialized.total_docs as u64,
            total_bytes: serialized.total_bytes as u64,
        })
    }
}

codegen_convex_serialization!(IndexBackfillMetadata, SerializedIndexBackfillMetadata);
