use std::{
    collections::BTreeSet,
    path::Path,
};

use anyhow::Context;
use common::{
    id_tracker::StaticIdTracker,
    persistence::DocumentRevisionStream,
};
use futures::TryStreamExt;
use tantivy::{
    directory::MmapDirectory,
    fastfield::AliveBitSet,
    DocAddress,
    Index,
    IndexBuilder,
    SingleSegmentIndexWriter,
};
use text_search::tracker::{
    MemoryDeletionTracker,
    SearchMemoryIdTracker,
};
use value::InternalId;

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

#[allow(dead_code)]
pub struct SearchSegment {
    pub segment: Index,
    pub alive_bitset: AliveBitSet,
    pub id_tracker: StaticIdTracker,
}

#[allow(dead_code)]
pub async fn merge_segments(search_segments: Vec<SearchSegment>, dir: &Path) -> anyhow::Result<()> {
    let mut segments = vec![];
    let settings = search_segments
        .first()
        .context("Called merge_segments with an empty vec of segments")?
        .segment
        .settings()
        .clone();
    for s in &search_segments {
        let mut index_segments = s.segment.searchable_segments()?;
        anyhow::ensure!(
            index_segments.len() == 1,
            "Expected exactly one segment, found {}",
            index_segments.len()
        );
        segments.push(index_segments.pop().unwrap());
    }
    let alive_bitsets = search_segments
        .iter()
        .map(|s| Some(s.alive_bitset.clone()))
        .collect::<Vec<_>>();
    let total_alive = search_segments
        .iter()
        .fold(0, |acc, e| acc + e.alive_bitset.num_alive_docs());

    let mmap_directory = MmapDirectory::open(dir)?;
    let (_merged_segment, id_mapping) =
        tantivy::merge_filtered_segments(&segments, settings, alive_bitsets, mmap_directory)?;
    anyhow::ensure!(
        total_alive == id_mapping.len(),
        "Total alive documents expected did not match merged segment id mapping"
    );
    let mut new_segment_id_tracker = SearchMemoryIdTracker::default();
    for (
        new_tantivy_id,
        DocAddress {
            segment_ord,
            doc_id: old_doc_id,
        },
    ) in id_mapping.iter_old_doc_addrs_enumerated()
    {
        let old_id_tracker = &search_segments[segment_ord as usize].id_tracker;
        let convex_id = old_id_tracker
            .get_convex_id(old_doc_id as usize)
            .with_context(|| {
                format!("Old id tracker for segment {segment_ord} is missing id {old_doc_id}")
            })?;
        new_segment_id_tracker.set_link(InternalId(convex_id), new_tantivy_id)?;
    }
    let num_docs = new_segment_id_tracker.num_ids();
    new_segment_id_tracker.write(dir.to_path_buf().join(ID_TRACKER_PATH))?;
    let tracker = MemoryDeletionTracker::new(num_docs as u32);
    tracker.write(
        dir.to_path_buf().join(ALIVE_BITSET_PATH),
        dir.to_path_buf().join(DELETED_TERMS_PATH),
    )?;
    Ok(())
}
