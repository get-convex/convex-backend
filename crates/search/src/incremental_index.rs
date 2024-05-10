use std::{
    collections::BTreeSet,
    path::Path,
};

use common::persistence::DocumentRevisionStream;
use futures::TryStreamExt;
use tantivy::{
    IndexBuilder,
    SingleSegmentIndexWriter,
};
use text_search::tracker::{
    MemoryDeletionTracker,
    SearchMemoryIdTracker,
};

use crate::{
    constants::CONVEX_EN_TOKENIZER,
    convex_en,
    TantivySearchIndexSchema,
};

/// The maximum size of a segment in bytes. 10MB.
#[allow(dead_code)]
const SEGMENT_MAX_SIZE_BYTES: usize = 10_000_000;

#[allow(dead_code)]
pub(crate) const ID_TRACKER_PATH: &str = "id_tracker";
#[allow(dead_code)]
pub(crate) const ALIVE_BITSET_PATH: &str = "tantivy_alive_bitset";
#[allow(dead_code)]
pub(crate) const DELETED_TERMS_PATH: &str = "deleted_terms";

#[allow(dead_code)]
pub async fn build_index(
    // Stream of document revisions in descending timestamp order.
    revision_stream: DocumentRevisionStream<'_>,
    tantivy_schema: TantivySearchIndexSchema,
    dir: &Path,
) -> anyhow::Result<()> {
    let index = IndexBuilder::new()
        .schema(tantivy_schema.schema.clone())
        .create_in_dir(dir)?;
    index
        .tokenizers()
        .register(CONVEX_EN_TOKENIZER, convex_en());
    let mut segment_writer = SingleSegmentIndexWriter::new(index, SEGMENT_MAX_SIZE_BYTES)?;
    let mut id_tracker = SearchMemoryIdTracker::default();
    futures::pin_mut!(revision_stream);
    // Keep track of the document IDs we've seen so we can check for duplicates.
    // We'll discard revisions to documents that we've already seen because we are
    // processing in reverse timestamp order.
    let mut document_ids_seen = BTreeSet::new();
    while let Some(revision_pair) = revision_stream.try_next().await? {
        let convex_id = revision_pair.id.internal_id();
        if document_ids_seen.contains(&convex_id) {
            continue;
        }
        document_ids_seen.insert(convex_id);
        if let Some(new_document) = revision_pair.document() {
            let tantivy_document =
                tantivy_schema.index_into_tantivy_document(new_document, revision_pair.ts());
            let doc_id = segment_writer.add_document(tantivy_document)?;
            id_tracker.set_link(convex_id, doc_id)?;
        }
    }
    segment_writer.finalize()?;
    id_tracker.write(dir.to_path_buf().join(ID_TRACKER_PATH))?;
    let tracker = MemoryDeletionTracker::new(document_ids_seen.len() as u32);
    tracker.write(
        dir.to_path_buf().join(ALIVE_BITSET_PATH),
        dir.to_path_buf().join(DELETED_TERMS_PATH),
    )?;
    Ok(())
}
