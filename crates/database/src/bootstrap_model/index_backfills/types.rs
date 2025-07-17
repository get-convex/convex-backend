use serde::{
    Deserialize,
    Serialize,
};
use value::{
    codegen_convex_serialization,
    DeveloperDocumentId,
};

/// Metadata for tracking index backfill progress.
///
/// This structure stores the progress of an index backfill operation,
/// tracking how many documents and bytes have been processed out of the total.
/// NB: We don't track the progress for catching up from the snapshot. This
/// should be relatively short, and we can show that we're not yet complete in
/// the UI. We can add timestamp-based progress for that phase in the future,
/// but number of documents and bytes is not possible to track because we walk
/// the revision stream.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct IndexBackfillMetadata {
    /// The ID of the index being backfilled. Should correspond to a document in
    /// the index table in `Backfilling` state.
    pub index_id: DeveloperDocumentId,
    /// Number of documents that have been indexed so far from the snapshot
    /// (does not include documents written since the backfill began)
    pub num_docs_indexed: u64,
    /// Total number of documents in the table from the snapshot
    /// (does not include documents written since the backfill began)
    pub total_docs: Option<u64>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedIndexBackfillMetadata {
    index_id: String,
    num_docs_indexed: i64,
    total_docs: Option<i64>,
}

impl From<IndexBackfillMetadata> for SerializedIndexBackfillMetadata {
    fn from(metadata: IndexBackfillMetadata) -> Self {
        SerializedIndexBackfillMetadata {
            index_id: metadata.index_id.to_string(),
            num_docs_indexed: metadata.num_docs_indexed as i64,
            total_docs: metadata.total_docs.map(|v| v as i64),
        }
    }
}

impl TryFrom<SerializedIndexBackfillMetadata> for IndexBackfillMetadata {
    type Error = anyhow::Error;

    fn try_from(serialized: SerializedIndexBackfillMetadata) -> Result<Self, Self::Error> {
        Ok(Self {
            index_id: serialized.index_id.parse()?,
            num_docs_indexed: serialized.num_docs_indexed as u64,
            total_docs: serialized.total_docs.map(|v| v as u64),
        })
    }
}

codegen_convex_serialization!(IndexBackfillMetadata, SerializedIndexBackfillMetadata);
