use std::sync::Arc;

use common::{
    bootstrap_model::index::{
        text_index::{
            FragmentedTextSegment,
            TextIndexSnapshot,
            TextIndexSnapshotData,
            TextIndexState,
            TextSnapshotVersion,
        },
        IndexConfig,
        IndexMetadata,
    },
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    knobs::SEARCHLIGHT_CLUSTER_NAME,
    query::{
        InternalSearch,
        InternalSearchFilterExpression,
        SearchVersion,
    },
    types::{
        IndexId,
        IndexName,
        PersistenceVersion,
        Timestamp,
        WriteTimestamp,
    },
};
use errors::ErrorMetadata;
use imbl::{
    ordmap::Entry,
    OrdMap,
};
use indexing::index_registry::{
    index_backfilling_error,
    Index,
    IndexRegistry,
};
use storage::Storage;

use crate::{
    memory_index::MemoryTextIndex,
    metrics,
    query::{
        CompiledQuery,
        RevisionWithKeys,
    },
    searcher::FragmentedTextStorageKeys,
    QueryResults,
    Searcher,
    TantivySearchIndexSchema,
};

#[derive(Clone)]
pub struct SnapshotInfo {
    pub disk_index: DiskIndex,
    pub disk_index_ts: Timestamp,
    pub disk_index_version: TextSnapshotVersion,
    pub memory_index: MemoryTextIndex,
}

#[derive(Clone)]
pub struct DiskIndex(Vec<FragmentedTextSegment>);

impl TryFrom<TextIndexSnapshotData> for DiskIndex {
    type Error = anyhow::Error;

    fn try_from(value: TextIndexSnapshotData) -> Result<Self, Self::Error> {
        Ok(match value {
            TextIndexSnapshotData::MultiSegment(segments) => Self(segments),
            TextIndexSnapshotData::Unknown(_) => anyhow::bail!("Unrecognized state: {:?}", value),
        })
    }
}

#[derive(Clone)]
pub enum TextIndex {
    Backfilling { memory_index: MemoryTextIndex },
    Backfilled(SnapshotInfo),
    Ready(SnapshotInfo),
}

impl TextIndex {
    fn memory_index(&self) -> &MemoryTextIndex {
        match self {
            TextIndex::Backfilling { ref memory_index } => memory_index,
            TextIndex::Backfilled(SnapshotInfo {
                ref memory_index, ..
            }) => memory_index,
            TextIndex::Ready(SnapshotInfo {
                ref memory_index, ..
            }) => memory_index,
        }
    }

    pub fn memory_index_mut(&mut self) -> &mut MemoryTextIndex {
        match self {
            TextIndex::Backfilling {
                ref mut memory_index,
            } => memory_index,
            TextIndex::Backfilled(SnapshotInfo {
                ref mut memory_index,
                ..
            }) => memory_index,
            TextIndex::Ready(SnapshotInfo {
                ref mut memory_index,
                ..
            }) => memory_index,
        }
    }
}

#[derive(Clone)]
pub struct TextIndexManager {
    indexes: TextIndexManagerState,
    persistence_version: PersistenceVersion,
}

#[derive(Clone)]
pub enum TextIndexManagerState {
    Bootstrapping,
    Ready(OrdMap<IndexId, TextIndex>),
}

impl TextIndexManager {
    pub fn is_bootstrapping(&self) -> bool {
        matches!(self.indexes, TextIndexManagerState::Bootstrapping)
    }

    pub fn new(indexes: TextIndexManagerState, persistence_version: PersistenceVersion) -> Self {
        Self {
            indexes,
            persistence_version,
        }
    }

    fn require_ready_indexes(&self) -> anyhow::Result<&OrdMap<IndexId, TextIndex>> {
        match self.indexes {
            TextIndexManagerState::Bootstrapping => {
                anyhow::bail!(ErrorMetadata::overloaded(
                    "SearchIndexesUnavailable",
                    "Search indexes bootstrapping and not yet available for use"
                ));
            },
            TextIndexManagerState::Ready(ref indexes) => Ok(indexes),
        }
    }

    fn get_snapshot_info(
        &self,
        index: &Index,
        printable_index_name: &IndexName,
    ) -> anyhow::Result<&SnapshotInfo> {
        let indexes = self.require_ready_indexes()?;
        let index = if let Some(index) = indexes.get(&index.id()) {
            index
        } else {
            anyhow::bail!("Search index {:?} not available", index.id());
        };

        match index {
            TextIndex::Backfilling { .. } | TextIndex::Backfilled(_) => {
                anyhow::bail!(index_backfilling_error(printable_index_name));
            },
            TextIndex::Ready(snapshot_info) => {
                anyhow::ensure!(
                    // If the search index was written to disk with a different format from
                    // how the current backend constructs search queries, assume the new
                    // search index is backfilling.
                    snapshot_info.disk_index_version
                        == TextSnapshotVersion::new(self.persistence_version),
                    index_backfilling_error(printable_index_name)
                );
                Ok(snapshot_info)
            },
        }
    }

    pub async fn search(
        &self,
        index: &Index,
        search: &InternalSearch,
        searcher: Arc<dyn Searcher>,
        search_storage: Arc<dyn Storage>,
        version: SearchVersion,
    ) -> anyhow::Result<QueryResults> {
        let timer = metrics::search_timer(&SEARCHLIGHT_CLUSTER_NAME);
        let tantivy_schema =
            TantivySearchIndexSchema::new_for_index(index, &search.printable_index_name()?)?;
        let (compiled_query, reads) = tantivy_schema.compile(search, version)?;
        // Ignore empty searches to avoid failures due to transient search issues (e.g.
        // bootstrapping). Do this after validating the query above.
        if search.filters.iter().any(|filter| {
            let InternalSearchFilterExpression::Search(_, query_string) = filter else {
                return false;
            };
            query_string.trim().is_empty()
        }) {
            tracing::debug!("Skipping empty search query");
            return Ok(QueryResults::empty());
        }

        let revisions_with_keys = self
            .run_compiled_query(
                index,
                &search.printable_index_name()?,
                tantivy_schema,
                compiled_query,
                searcher,
                search_storage,
            )
            .await?;

        let results = QueryResults {
            revisions_with_keys,
            reads,
        };
        metrics::finish_search(timer, &results.revisions_with_keys);
        Ok(results)
    }

    pub async fn search_with_compiled_query(
        &self,
        index: &Index,
        printable_index_name: &IndexName,
        query: pb::searchlight::TextQuery,
        searcher: Arc<dyn Searcher>,
        search_storage: Arc<dyn Storage>,
    ) -> anyhow::Result<RevisionWithKeys> {
        let timer = metrics::search_timer(&SEARCHLIGHT_CLUSTER_NAME);
        let tantivy_schema = TantivySearchIndexSchema::new_for_index(index, printable_index_name)?;
        let compiled_query =
            CompiledQuery::try_from_text_query_proto(query, tantivy_schema.search_field)?;

        let revisions_with_keys = self
            .run_compiled_query(
                index,
                printable_index_name,
                tantivy_schema,
                compiled_query,
                searcher,
                search_storage,
            )
            .await?;
        metrics::finish_search(timer, &revisions_with_keys);
        Ok(revisions_with_keys)
    }

    async fn run_compiled_query(
        &self,
        index: &Index,
        printable_index_name: &IndexName,
        tantivy_schema: TantivySearchIndexSchema,
        compiled_query: CompiledQuery,
        searcher: Arc<dyn Searcher>,
        search_storage: Arc<dyn Storage>,
    ) -> anyhow::Result<RevisionWithKeys> {
        let SnapshotInfo {
            disk_index,
            disk_index_ts,
            memory_index,
            ..
        } = self.get_snapshot_info(index, printable_index_name)?;
        tantivy_schema
            .search(
                compiled_query,
                memory_index,
                search_storage,
                disk_index
                    .0
                    .iter()
                    .cloned()
                    .map(FragmentedTextStorageKeys::from)
                    .collect(),
                *disk_index_ts,
                searcher,
            )
            .await
    }

    pub fn backfilled_and_enabled_index_sizes(
        &self,
    ) -> anyhow::Result<impl Iterator<Item = (IndexId, usize)> + '_> {
        Ok(self
            .require_ready_indexes()?
            .iter()
            .filter_map(|(id, idx)| {
                let SnapshotInfo { memory_index, .. } = match idx {
                    TextIndex::Backfilled(snapshot) => snapshot,
                    TextIndex::Ready(snapshot) => snapshot,
                    TextIndex::Backfilling { .. } => return None,
                };
                Some((*id, memory_index.size()))
            }))
    }

    pub fn num_transactions(&self, index_id: IndexId) -> anyhow::Result<Option<usize>> {
        let Some(index) = self.require_ready_indexes()?.get(&index_id) else {
            return Ok(None);
        };
        let SnapshotInfo { memory_index, .. } = match index {
            TextIndex::Ready(snapshot) => snapshot,
            TextIndex::Backfilled(snapshot) => snapshot,
            TextIndex::Backfilling { .. } => return Ok(None),
        };

        Ok(Some(memory_index.num_transactions()))
    }

    pub fn update(
        &mut self,
        index_registry: &IndexRegistry,
        deletion: Option<&ResolvedDocument>,
        insertion: Option<&ResolvedDocument>,
        ts: WriteTimestamp,
    ) -> anyhow::Result<()> {
        let TextIndexManagerState::Ready(ref mut indexes) = self.indexes else {
            return Ok(());
        };
        let Some(id) = deletion.as_ref().or(insertion.as_ref()).map(|d| d.id()) else {
            return Ok(());
        };
        let timer = metrics::index_manager_update_timer();

        // Handle index maintenance operations first.
        // TODO: Move this validation to `IndexRegistry` and make this function
        // infallible.
        if id.tablet_id == index_registry.index_table() {
            match (deletion, insertion) {
                (None, Some(insertion)) => {
                    let metadata = IndexMetadata::try_from(insertion.value().clone().0)?;
                    if let IndexConfig::Text {
                        ref on_disk_state, ..
                    } = metadata.config
                    {
                        let TextIndexState::Backfilling(_) = on_disk_state else {
                            anyhow::bail!(
                                "Inserted new search index that wasn't backfilling: {metadata:?}"
                            );
                        };
                        let memory_index = MemoryTextIndex::new(ts);
                        let index = TextIndex::Backfilling { memory_index };
                        indexes.insert(insertion.id().internal_id(), index);

                        metrics::log_index_created();
                    }
                },
                (Some(prev_version), Some(next_version)) => {
                    let prev_metadata: ParsedDocument<IndexMetadata<_>> =
                        prev_version.clone().try_into()?;
                    let next_metadata: ParsedDocument<IndexMetadata<_>> =
                        next_version.clone().try_into()?;
                    let (old_snapshot, new_snapshot) =
                        match (&prev_metadata.config, &next_metadata.config) {
                            (
                                IndexConfig::Text {
                                    on_disk_state: TextIndexState::Backfilling { .. },
                                    ..
                                },
                                IndexConfig::Text {
                                    on_disk_state: TextIndexState::Backfilling { .. },
                                    ..
                                },
                            ) => (None, None),
                            (
                                IndexConfig::Text {
                                    on_disk_state: TextIndexState::Backfilling { .. },
                                    ..
                                },
                                IndexConfig::Text {
                                    on_disk_state: TextIndexState::Backfilled(snapshot),
                                    ..
                                },
                            ) => (None, Some(snapshot)),
                            (
                                IndexConfig::Text {
                                    on_disk_state: TextIndexState::Backfilled(old_snapshot),
                                    ..
                                },
                                IndexConfig::Text {
                                    on_disk_state: TextIndexState::SnapshottedAt(new_snapshot),
                                    ..
                                },
                            ) => (Some(old_snapshot), Some(new_snapshot)),
                            (
                                IndexConfig::Text {
                                    on_disk_state: TextIndexState::Backfilled(old_snapshot),
                                    ..
                                },
                                IndexConfig::Text {
                                    on_disk_state: TextIndexState::Backfilled(new_snapshot),
                                    ..
                                },
                            ) => (Some(old_snapshot), Some(new_snapshot)),
                            (
                                IndexConfig::Text {
                                    on_disk_state: TextIndexState::SnapshottedAt(old_snapshot),
                                    ..
                                },
                                IndexConfig::Text {
                                    on_disk_state: TextIndexState::SnapshottedAt(new_snapshot),
                                    ..
                                },
                            ) => (Some(old_snapshot), Some(new_snapshot)),
                            (IndexConfig::Text { .. }, _) | (_, IndexConfig::Text { .. }) => {
                                anyhow::bail!(
                                    "Invalid index type transition: {prev_metadata:?} to \
                                     {next_metadata:?}"
                                );
                            },
                            _ => (None, None),
                        };
                    if let Some(TextIndexSnapshot {
                        data: disk_index,
                        ts: disk_index_ts,
                        version: disk_index_version,
                    }) = new_snapshot
                    {
                        let is_newly_enabled =
                            !prev_metadata.config.is_enabled() && next_metadata.config.is_enabled();
                        let is_updated_snapshot = if let Some(old_snapshot) = old_snapshot {
                            old_snapshot.ts < *disk_index_ts
                        } else {
                            true
                        };

                        if is_newly_enabled || is_updated_snapshot {
                            let mut entry = match indexes.entry(id.internal_id()) {
                                Entry::Occupied(e) => e,
                                Entry::Vacant(..) => anyhow::bail!("Missing index for {id}"),
                            };
                            let memory_index = match entry.get() {
                                TextIndex::Backfilling { memory_index } => memory_index,
                                TextIndex::Backfilled(SnapshotInfo { memory_index, .. }) => {
                                    memory_index
                                },
                                TextIndex::Ready(SnapshotInfo { memory_index, .. }) => memory_index,
                            };

                            if let Some(old_snapshot) = old_snapshot {
                                anyhow::ensure!(old_snapshot.ts <= disk_index_ts);
                            } else {
                                anyhow::ensure!(
                                    memory_index.min_ts()
                                        <= WriteTimestamp::Committed(*disk_index_ts)
                                );
                            }

                            let disk_index = DiskIndex::try_from(disk_index.clone())?;
                            let mut memory_index = memory_index.clone();
                            memory_index.truncate(disk_index_ts.succ()?)?;
                            let snapshot = SnapshotInfo {
                                disk_index,
                                disk_index_ts: *disk_index_ts,
                                disk_index_version: *disk_index_version,
                                memory_index,
                            };

                            let is_next_index_enabled =
                                next_metadata.into_value().config.is_enabled();
                            *entry.get_mut() = if is_next_index_enabled {
                                TextIndex::Ready(snapshot)
                            } else {
                                TextIndex::Backfilled(snapshot)
                            };

                            if !prev_metadata.into_value().config.is_enabled()
                                && is_next_index_enabled
                            {
                                metrics::log_index_backfilled();
                            } else {
                                metrics::log_index_advanced();
                            }
                        }
                    }
                },
                (Some(deletion), None) => {
                    let metadata: ParsedDocument<IndexMetadata<_>> = deletion.clone().try_into()?;
                    if metadata.is_text_index() {
                        indexes.remove(&deletion.id().internal_id());
                        metrics::log_index_deleted();
                    }
                },
                _ => panic!("Had neither a deletion nor insertion despite checking above"),
            }
        }

        // Handle index updates for our existing search indexes.
        for index in index_registry.text_indexes_by_table(id.tablet_id) {
            let IndexConfig::Text {
                ref developer_config,
                ..
            } = index.metadata.config
            else {
                continue;
            };
            let tantivy_schema = TantivySearchIndexSchema::new(developer_config);
            let Some(index) = indexes.get_mut(&index.id()) else {
                continue;
            };
            let old_value = deletion
                .as_ref()
                .map(|d| {
                    anyhow::Ok((
                        tantivy_schema.index_into_terms(d)?,
                        d.creation_time()
                            .expect("Document should have creation time"),
                    ))
                })
                .transpose()?;
            let new_terms = insertion
                .as_ref()
                .map(|d| {
                    anyhow::Ok((
                        tantivy_schema.index_into_terms(d)?,
                        d.creation_time()
                            .expect("Document should have creation time"),
                    ))
                })
                .transpose()?;
            index
                .memory_index_mut()
                .update(id.internal_id(), ts, old_value, new_terms)?;
        }

        timer.finish();
        Ok(())
    }

    pub fn total_in_memory_size(&self) -> usize {
        self.in_memory_sizes().iter().map(|(_, s)| s).sum()
    }

    pub fn in_memory_sizes(&self) -> Vec<(IndexId, usize)> {
        match self.indexes {
            TextIndexManagerState::Bootstrapping => vec![],
            TextIndexManagerState::Ready(ref indexes) => indexes
                .iter()
                .map(|(id, s)| (*id, s.memory_index().size()))
                .collect(),
        }
    }

    pub fn consistency_check(&self) -> anyhow::Result<()> {
        let indexes = self.require_ready_indexes()?;
        for index in indexes.values() {
            index.memory_index().consistency_check()?;
        }
        Ok(())
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn ready_indexes(&self) -> &OrdMap<IndexId, TextIndex> {
        match self.indexes {
            TextIndexManagerState::Bootstrapping => panic!("Search indexes not ready"),
            TextIndexManagerState::Ready(ref indexes) => indexes,
        }
    }
}
