use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    path::PathBuf,
    sync::Arc,
};

use anyhow::Context;
use async_trait::async_trait;
use common::{
    bootstrap_model::index::{
        text_index::{
            DeveloperTextIndexConfig,
            FragmentedTextSegment,
            TextBackfillCursor,
            TextIndexBackfillState,
            TextIndexSnapshot,
            TextIndexSnapshotData,
            TextIndexState,
            TextSnapshotVersion,
        },
        IndexConfig,
        TabletIndexMetadata,
    },
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    persistence::{
        DocumentStream,
        RepeatablePersistence,
    },
    persistence_helpers::{
        stream_revision_pairs,
        DocumentRevision,
        RevisionPair,
    },
    query::Order,
    runtime::{
        try_join_buffer_unordered,
        Runtime,
    },
    types::IndexId,
};
use futures::{
    StreamExt,
    TryStreamExt,
};
use search::{
    build_new_segment,
    disk_index::upload_text_segment,
    fetch_term_ordinals_and_remap_deletes,
    metrics::SearchType,
    searcher::{
        FragmentedTextStorageKeys,
        SegmentTermMetadataFetcher,
    },
    NewTextSegment,
    PreviousTextSegments,
    Searcher,
    SegmentStatisticsUpdates,
    TantivySearchIndexSchema,
    UpdatableTextSegment,
};
use storage::Storage;
use sync_types::Timestamp;

use crate::{
    index_workers::{
        index_meta::{
            BackfillState,
            SearchIndex,
            SearchIndexConfig,
            SearchOnDiskState,
            SearchSnapshot,
            SegmentStatistics,
            SegmentType,
            SnapshotData,
        },
        search_flusher::MultipartBuildType,
    },
    Snapshot,
};

#[derive(Clone, Debug)]
pub struct TextSearchIndex;

impl SegmentType<TextSearchIndex> for FragmentedTextSegment {
    fn id(&self) -> &str {
        &self.id
    }

    fn statistics(&self) -> anyhow::Result<TextStatistics> {
        Ok(TextStatistics {
            num_indexed_documents: self.num_indexed_documents,
            num_deleted_documents: self.num_deleted_documents,
        })
    }

    fn total_size_bytes(
        &self,
        _config: &<TextSearchIndex as SearchIndex>::DeveloperConfig,
    ) -> anyhow::Result<u64> {
        Ok(self.size_bytes_total)
    }
}
#[derive(Clone)]
pub struct BuildTextIndexArgs {
    pub search_storage: Arc<dyn Storage>,
    pub segment_term_metadata_fetcher: Arc<dyn SegmentTermMetadataFetcher>,
}

#[async_trait]
impl SearchIndex for TextSearchIndex {
    type BuildIndexArgs = BuildTextIndexArgs;
    type DeveloperConfig = DeveloperTextIndexConfig;
    type NewSegment = NewTextSegment;
    type PreviousSegments = PreviousTextSegments;
    type Schema = TantivySearchIndexSchema;
    type Segment = FragmentedTextSegment;
    type Statistics = TextStatistics;

    fn get_config(config: IndexConfig) -> Option<SearchIndexConfig<Self>> {
        let IndexConfig::Search {
            on_disk_state,
            developer_config,
        } = config
        else {
            return None;
        };
        Some(SearchIndexConfig {
            developer_config,
            on_disk_state: match on_disk_state {
                TextIndexState::Backfilling(snapshot) => {
                    SearchOnDiskState::Backfilling(snapshot.into())
                },
                TextIndexState::Backfilled(snapshot) => {
                    SearchOnDiskState::Backfilled(snapshot.into())
                },
                TextIndexState::SnapshottedAt(snapshot) => {
                    SearchOnDiskState::SnapshottedAt(snapshot.into())
                },
            },
        })
    }

    // When iterating over the document log for partial segments, we must iterate in
    // reverse timestamp order to match assumptions made in build_disk_index
    // that allow for greater efficiency.
    fn partial_document_order() -> Order {
        Order::Desc
    }

    fn get_index_sizes<RT: Runtime>(
        snapshot: Snapshot<RT>,
    ) -> anyhow::Result<BTreeMap<IndexId, usize>> {
        Ok(snapshot
            .search_indexes
            .backfilled_and_enabled_index_sizes()?
            .collect())
    }

    fn is_version_current(snapshot: &SearchSnapshot<Self>) -> bool {
        // TODO(sam): This doesn't match the current persistence version based check,
        // but it's closer to what vector search does.
        snapshot.data.is_version_current()
    }

    fn new_schema(config: &Self::DeveloperConfig) -> Self::Schema {
        TantivySearchIndexSchema::new(config)
    }

    async fn download_previous_segments<RT: Runtime>(
        rt: &RT,
        storage: Arc<dyn Storage>,
        segments: Vec<Self::Segment>,
    ) -> anyhow::Result<Self::PreviousSegments> {
        let segments: Vec<_> = try_join_buffer_unordered(
            rt,
            "download_text_meta",
            segments
                .into_iter()
                .map(move |segment| UpdatableTextSegment::download(segment, storage.clone())),
        )
        .await?;
        let segments = segments
            .into_iter()
            .map(|updatable_segment| (updatable_segment.segment_key().clone(), updatable_segment))
            .collect();
        Ok(PreviousTextSegments(segments))
    }

    async fn upload_previous_segments<RT: Runtime>(
        rt: &RT,
        storage: Arc<dyn Storage>,
        segments: Self::PreviousSegments,
    ) -> anyhow::Result<Vec<Self::Segment>> {
        try_join_buffer_unordered(
            rt,
            "upload_text_metadata",
            segments
                .0
                .into_values()
                .map(move |segment| segment.upload_metadata(storage.clone())),
        )
        .await
    }

    fn estimate_document_size(schema: &Self::Schema, doc: &ResolvedDocument) -> u64 {
        schema.estimate_size(doc)
    }

    async fn build_disk_index<RT: Runtime>(
        rt: &RT,
        schema: &Self::Schema,
        index_path: &PathBuf,
        documents: DocumentStream<'_>,
        reader: RepeatablePersistence,
        previous_segments: &mut Self::PreviousSegments,
        lower_bound_ts: Option<Timestamp>,
        BuildTextIndexArgs {
            search_storage,
            segment_term_metadata_fetcher,
        }: BuildTextIndexArgs,
        multipart_build_type: MultipartBuildType,
    ) -> anyhow::Result<Option<Self::NewSegment>> {
        let revision_stream = match multipart_build_type {
            MultipartBuildType::Partial(_) => Box::pin(stream_revision_pairs(documents, &reader)),
            // Create a fake revision stream for complete builds because we are building from
            // scratch so we don't need to look up previous revisions. We know there are no deletes.
            MultipartBuildType::Complete | MultipartBuildType::IncrementalComplete { .. } => {
                documents
                    .map(|result| {
                        let (ts, id, maybe_doc) = result?;
                        anyhow::ensure!(maybe_doc.is_some(), "Document must exist");
                        Ok(RevisionPair {
                            id,
                            rev: DocumentRevision {
                                ts,
                                document: maybe_doc,
                            },
                            prev_rev: None,
                        })
                    })
                    .boxed()
            },
        };

        build_new_segment(
            rt,
            revision_stream,
            schema.clone(),
            index_path,
            previous_segments,
            segment_term_metadata_fetcher,
            search_storage,
            lower_bound_ts,
        )
        .await
    }

    async fn upload_new_segment<RT: Runtime>(
        rt: &RT,
        storage: Arc<dyn Storage>,
        new_segment: Self::NewSegment,
    ) -> anyhow::Result<Self::Segment> {
        upload_text_segment(rt, storage, new_segment).await
    }

    fn extract_metadata(
        metadata: ParsedDocument<TabletIndexMetadata>,
    ) -> anyhow::Result<(Self::DeveloperConfig, SearchOnDiskState<Self>)> {
        let (on_disk_state, developer_config) = match metadata.into_value().config {
            IndexConfig::Database { .. } | IndexConfig::Vector { .. } => {
                anyhow::bail!("Index type changed!")
            },
            IndexConfig::Search {
                developer_config,
                on_disk_state,
            } => (on_disk_state, developer_config),
        };
        Ok((developer_config, SearchOnDiskState::from(on_disk_state)))
    }

    fn new_index_config(
        developer_config: Self::DeveloperConfig,
        new_state: SearchOnDiskState<Self>,
    ) -> anyhow::Result<IndexConfig> {
        let on_disk_state = TextIndexState::from(new_state);
        Ok(IndexConfig::Search {
            on_disk_state,
            developer_config,
        })
    }

    fn search_type() -> SearchType {
        SearchType::Text
    }

    async fn execute_compaction(
        searcher: Arc<dyn Searcher>,
        search_storage: Arc<dyn Storage>,
        _config: &Self::DeveloperConfig,
        segments: Vec<Self::Segment>,
    ) -> anyhow::Result<Self::Segment> {
        searcher
            .execute_text_compaction(
                search_storage,
                segments
                    .into_iter()
                    .map(FragmentedTextStorageKeys::from)
                    .collect(),
            )
            .await
    }

    async fn merge_deletes<RT: Runtime>(
        runtime: &RT,
        previous_segments: &mut Self::PreviousSegments,
        documents: DocumentStream<'_>,
        repeatable_persistence: &RepeatablePersistence,
        build_index_args: Self::BuildIndexArgs,
        schema: Self::Schema,
        document_log_lower_bound: Timestamp,
    ) -> anyhow::Result<()> {
        let revision_stream = stream_revision_pairs(documents, repeatable_persistence);
        // Keep track of the document IDs we've either added to our new segment or
        // deleted from a previous segment. Because we process in reverse order, we
        // may encounter each document id multiple times, but we only want to add or
        // delete them once.
        let mut document_ids_processed = BTreeSet::new();
        // Deleted documents that either have a revision in a previous segment that we
        // will eventually encounter in the log and delete. Or that were added and
        // deleted within our new segment's time window and can be ignored.
        let mut dangling_deletes = BTreeSet::new();
        let mut segment_statistics_updates = SegmentStatisticsUpdates::new();
        futures::pin_mut!(revision_stream);
        while let Some(revision_pair) = revision_stream.try_next().await? {
            // We need to pass the schema through here.
            // Update segment statistics
            // For each document, three possible outcomes:
            // 1. We add the document to our new segment
            // 2. We delete the document from a previous segment
            // 3. We ignore the document because it was both added and removed within the
            //    time bounds for our new segment.
            let convex_id = revision_pair.id.internal_id();

            // Skip documents we have already added to the segment, but update dangling
            // deletes
            if document_ids_processed.contains(&convex_id) {
                continue;
            }
            document_ids_processed.insert(convex_id);

            // Addition
            if let Some(_new_document) = revision_pair.document() {
                // If we have already processed a delete for this document at a higher
                // timestamp, we can ignore it. Otherwise, add it to the segment.
                if dangling_deletes.contains(&convex_id) {
                    dangling_deletes.remove(&convex_id);
                }
            }

            // Delete
            if let Some(prev_rev) = revision_pair.prev_rev
                && let Some(prev_document) = prev_rev.document.as_ref()
            {
                if prev_rev.ts > document_log_lower_bound {
                    // This document might be an add, or might be replaced earlier in the log,
                    // we don't know, so we need to process it again
                    // later.
                    dangling_deletes.insert(prev_document.id().internal_id());
                    document_ids_processed.remove(&prev_document.id().internal_id());
                } else {
                    let segment_key = previous_segments
                        .delete_document(prev_document.id().internal_id())?
                        .context(
                            "Missing segment even though revision is not in our time bounds",
                        )?;
                    let segment = previous_segments
                        .0
                        .get(&segment_key)
                        .context("Segment key not found")?;

                    let terms = schema.index_into_terms(prev_document)?;
                    segment_statistics_updates
                        .on_document_deleted(&segment.original.segment_key, terms);
                }
            }
        }
        anyhow::ensure!(
            dangling_deletes.is_empty(),
            "Dangling deletes is not empty. A document was deleted that is not present in other \
             segments nor in this stream"
        );

        let segments_term_metadata = fetch_term_ordinals_and_remap_deletes(
            runtime,
            build_index_args.search_storage.clone(),
            build_index_args.segment_term_metadata_fetcher.clone(),
            segment_statistics_updates.term_deletes_by_segment,
        )
        .await?;
        previous_segments.update_term_deletion_metadata(segments_term_metadata)?;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct TextStatistics {
    pub num_indexed_documents: u64,
    pub num_deleted_documents: u64,
}

impl From<SearchOnDiskState<TextSearchIndex>> for TextIndexState {
    fn from(value: SearchOnDiskState<TextSearchIndex>) -> Self {
        match value {
            SearchOnDiskState::Backfilling(state) => Self::Backfilling(state.into()),
            SearchOnDiskState::Backfilled(snapshot) => Self::Backfilled(snapshot.into()),
            SearchOnDiskState::SnapshottedAt(snapshot) => Self::SnapshottedAt(snapshot.into()),
        }
    }
}

impl From<TextIndexState> for SearchOnDiskState<TextSearchIndex> {
    fn from(value: TextIndexState) -> Self {
        match value {
            TextIndexState::Backfilling(state) => Self::Backfilling(state.into()),
            TextIndexState::Backfilled(snapshot) => Self::Backfilled(snapshot.into()),
            TextIndexState::SnapshottedAt(snapshot) => Self::SnapshottedAt(snapshot.into()),
        }
    }
}

impl SegmentStatistics for TextStatistics {
    fn add(lhs: anyhow::Result<Self>, rhs: anyhow::Result<Self>) -> anyhow::Result<Self> {
        let lhs = lhs?;
        let rhs = rhs?;
        Ok(Self {
            num_indexed_documents: lhs.num_indexed_documents + rhs.num_indexed_documents,
            num_deleted_documents: lhs.num_deleted_documents + rhs.num_deleted_documents,
        })
    }

    fn num_documents(&self) -> u64 {
        self.num_indexed_documents
    }

    fn num_non_deleted_documents(&self) -> u64 {
        self.num_indexed_documents - self.num_deleted_documents
    }
}

impl From<TextIndexBackfillState> for BackfillState<TextSearchIndex> {
    fn from(value: TextIndexBackfillState) -> Self {
        Self {
            segments: value.segments,
            cursor: value.cursor.clone().map(|value| value.cursor),
            backfill_snapshot_ts: value.cursor.map(|value| value.backfill_snapshot_ts),
        }
    }
}

impl From<BackfillState<TextSearchIndex>> for TextIndexBackfillState {
    fn from(value: BackfillState<TextSearchIndex>) -> Self {
        let cursor = if let Some(cursor) = value.cursor
            && let Some(backfill_snapshot_ts) = value.backfill_snapshot_ts
        {
            Some(TextBackfillCursor {
                cursor,
                backfill_snapshot_ts,
            })
        } else {
            None
        };
        Self {
            segments: value.segments,
            cursor,
        }
    }
}

impl From<TextIndexSnapshot> for SearchSnapshot<TextSearchIndex> {
    fn from(snapshot: TextIndexSnapshot) -> Self {
        Self {
            ts: snapshot.ts,
            data: snapshot.data.into(),
        }
    }
}

impl From<SearchSnapshot<TextSearchIndex>> for TextIndexSnapshot {
    fn from(value: SearchSnapshot<TextSearchIndex>) -> Self {
        Self {
            ts: value.ts,
            data: value.data.into(),
            version: TextSnapshotVersion::V2UseStringIds,
        }
    }
}

impl From<SnapshotData<FragmentedTextSegment>> for TextIndexSnapshotData {
    fn from(value: SnapshotData<FragmentedTextSegment>) -> Self {
        match value {
            SnapshotData::Unknown(obj) => TextIndexSnapshotData::Unknown(obj),
            SnapshotData::SingleSegment(key) => TextIndexSnapshotData::SingleSegment(key),
            SnapshotData::MultiSegment(segments) => TextIndexSnapshotData::MultiSegment(segments),
        }
    }
}

impl From<TextIndexSnapshotData> for SnapshotData<FragmentedTextSegment> {
    fn from(value: TextIndexSnapshotData) -> Self {
        match value {
            TextIndexSnapshotData::SingleSegment(key) => SnapshotData::SingleSegment(key),
            TextIndexSnapshotData::Unknown(obj) => SnapshotData::Unknown(obj),
            TextIndexSnapshotData::MultiSegment(segments) => SnapshotData::MultiSegment(segments),
        }
    }
}
