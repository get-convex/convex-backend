use std::sync::Arc;

use common::{
    bootstrap_model::index::{
        search_index::{
            SearchIndexSnapshot,
            SearchIndexState,
            SearchSnapshotVersion,
        },
        IndexConfig,
        IndexMetadata,
    },
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    query::{
        InternalSearch,
        InternalSearchFilterExpression,
        SearchVersion,
    },
    types::{
        IndexId,
        IndexName,
        ObjectKey,
        PersistenceVersion,
        Timestamp,
        WriteTimestamp,
    },
};
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
    memory_index::MemorySearchIndex,
    metrics,
    query::{
        CompiledQuery,
        RevisionWithKeys,
    },
    QueryResults,
    Searcher,
    TantivySearchIndexSchema,
};

#[derive(Clone)]
pub struct SnapshotInfo {
    pub disk_index: ObjectKey,
    pub disk_index_ts: Timestamp,
    pub disk_index_version: SearchSnapshotVersion,
    pub memory_index: MemorySearchIndex,
}

#[derive(Clone)]
pub enum SearchIndex {
    Backfilling { memory_index: MemorySearchIndex },
    Backfilled(SnapshotInfo),
    Ready(SnapshotInfo),
}

impl SearchIndex {
    fn memory_index(&self) -> &MemorySearchIndex {
        match self {
            SearchIndex::Backfilling { ref memory_index } => memory_index,
            SearchIndex::Backfilled(SnapshotInfo {
                ref memory_index, ..
            }) => memory_index,
            SearchIndex::Ready(SnapshotInfo {
                ref memory_index, ..
            }) => memory_index,
        }
    }

    pub fn memory_index_mut(&mut self) -> &mut MemorySearchIndex {
        match self {
            SearchIndex::Backfilling {
                ref mut memory_index,
            } => memory_index,
            SearchIndex::Backfilled(SnapshotInfo {
                ref mut memory_index,
                ..
            }) => memory_index,
            SearchIndex::Ready(SnapshotInfo {
                ref mut memory_index,
                ..
            }) => memory_index,
        }
    }
}

#[derive(Clone)]
pub struct SearchIndexManager {
    indexes: OrdMap<IndexId, SearchIndex>,
    persistence_version: PersistenceVersion,
}

impl SearchIndexManager {
    pub fn from_bootstrap(
        indexes: OrdMap<IndexId, SearchIndex>,
        persistence_version: PersistenceVersion,
    ) -> Self {
        Self {
            indexes,
            persistence_version,
        }
    }

    fn get_snapshot_info(
        &self,
        index: &Index,
        printable_index_name: &IndexName,
    ) -> anyhow::Result<&SnapshotInfo> {
        let index = if let Some(index) = self.indexes.get(&index.id()) {
            index
        } else {
            anyhow::bail!("Search index {:?} not available", index.id());
        };

        match index {
            SearchIndex::Backfilling { .. } | SearchIndex::Backfilled(_) => {
                anyhow::bail!(index_backfilling_error(printable_index_name));
            },
            SearchIndex::Ready(snapshot_info) => {
                anyhow::ensure!(
                    // If the search index was written to disk with a different format from
                    // how the current backend constructs search queries, assume the new
                    // search index is backfilling.
                    snapshot_info.disk_index_version
                        == SearchSnapshotVersion::new(self.persistence_version),
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
        let timer = metrics::search_timer();
        let tantivy_schema =
            TantivySearchIndexSchema::new_for_index(index, &search.printable_index_name()?)?;

        let SnapshotInfo {
            disk_index,
            disk_index_ts,
            memory_index,
            ..
        } = self.get_snapshot_info(index, &search.printable_index_name()?)?;
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

        let revisions_with_keys = tantivy_schema
            .search(
                compiled_query,
                memory_index,
                search_storage,
                disk_index,
                *disk_index_ts,
                searcher,
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
        let timer = metrics::search_timer();
        let tantivy_schema = TantivySearchIndexSchema::new_for_index(index, printable_index_name)?;
        let compiled_query =
            CompiledQuery::try_from_text_query_proto(query, tantivy_schema.search_field)?;

        let SnapshotInfo {
            disk_index,
            disk_index_ts,
            memory_index,
            ..
        } = self.get_snapshot_info(index, printable_index_name)?;

        // Convert the ObjectKey to an absolute path.
        let revisions_with_keys = tantivy_schema
            .search(
                compiled_query,
                memory_index,
                search_storage,
                disk_index,
                *disk_index_ts,
                searcher,
            )
            .await?;
        metrics::finish_search(timer, &revisions_with_keys);
        Ok(revisions_with_keys)
    }

    pub fn backfilled_and_enabled_index_sizes(
        &self,
    ) -> impl Iterator<Item = (IndexId, usize)> + '_ {
        self.indexes.iter().filter_map(|(id, idx)| {
            let SnapshotInfo { memory_index, .. } = match idx {
                SearchIndex::Backfilled(snapshot) => snapshot,
                SearchIndex::Ready(snapshot) => snapshot,
                SearchIndex::Backfilling { .. } => return None,
            };
            Some((*id, memory_index.size()))
        })
    }

    pub fn num_transactions(&self, index_id: IndexId) -> Option<usize> {
        let Some(index) = self.indexes.get(&index_id) else {
            return None;
        };
        let SnapshotInfo { memory_index, .. } = match index {
            SearchIndex::Ready(snapshot) => snapshot,
            SearchIndex::Backfilled(snapshot) => snapshot,
            SearchIndex::Backfilling { .. } => return None,
        };

        Some(memory_index.num_transactions())
    }

    pub fn update(
        &mut self,
        index_registry: &IndexRegistry,
        deletion: Option<&ResolvedDocument>,
        insertion: Option<&ResolvedDocument>,
        ts: WriteTimestamp,
    ) -> anyhow::Result<()> {
        let Some(id) = deletion.as_ref().or(insertion.as_ref()).map(|d| d.id()) else {
            return Ok(());
        };
        let timer = metrics::index_manager_update_timer();

        // Handle index maintenance operations first.
        // TODO: Move this validation to `IndexRegistry` and make this function
        // infallible.
        if *id.table() == index_registry.index_table() {
            match (deletion, insertion) {
                (None, Some(insertion)) => {
                    let metadata = IndexMetadata::try_from(insertion.value().clone().0)?;
                    if let IndexConfig::Search {
                        ref on_disk_state, ..
                    } = metadata.config
                    {
                        let SearchIndexState::Backfilling = on_disk_state else {
                            anyhow::bail!(
                                "Inserted new search index that wasn't backfilling: {metadata:?}"
                            );
                        };
                        let memory_index = MemorySearchIndex::new(ts);
                        let index = SearchIndex::Backfilling { memory_index };
                        self.indexes.insert(insertion.id().internal_id(), index);

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
                                IndexConfig::Search {
                                    on_disk_state: SearchIndexState::Backfilling { .. },
                                    ..
                                },
                                IndexConfig::Search {
                                    on_disk_state: SearchIndexState::Backfilled(snapshot),
                                    ..
                                },
                            ) => (None, Some(snapshot)),
                            (
                                IndexConfig::Search {
                                    on_disk_state: SearchIndexState::Backfilled(old_snapshot),
                                    ..
                                },
                                IndexConfig::Search {
                                    on_disk_state: SearchIndexState::SnapshottedAt(new_snapshot),
                                    ..
                                },
                            ) => (Some(old_snapshot), Some(new_snapshot)),
                            (
                                IndexConfig::Search {
                                    on_disk_state: SearchIndexState::Backfilled(old_snapshot),
                                    ..
                                },
                                IndexConfig::Search {
                                    on_disk_state: SearchIndexState::Backfilled(new_snapshot),
                                    ..
                                },
                            ) => (Some(old_snapshot), Some(new_snapshot)),
                            (
                                IndexConfig::Search {
                                    on_disk_state: SearchIndexState::SnapshottedAt(old_snapshot),
                                    ..
                                },
                                IndexConfig::Search {
                                    on_disk_state: SearchIndexState::SnapshottedAt(new_snapshot),
                                    ..
                                },
                            ) => (Some(old_snapshot), Some(new_snapshot)),
                            (IndexConfig::Search { .. }, _) | (_, IndexConfig::Search { .. }) => {
                                anyhow::bail!(
                                    "Invalid index type transition: {prev_metadata:?} to \
                                     {next_metadata:?}"
                                );
                            },
                            _ => (None, None),
                        };
                    if let Some(SearchIndexSnapshot {
                        index: disk_index,
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
                            let mut entry = match self.indexes.entry(id.internal_id()) {
                                Entry::Occupied(e) => e,
                                Entry::Vacant(..) => anyhow::bail!("Missing index for {id}"),
                            };
                            let memory_index = match entry.get() {
                                SearchIndex::Backfilling { memory_index } => memory_index,
                                SearchIndex::Backfilled(SnapshotInfo { memory_index, .. }) => {
                                    memory_index
                                },
                                SearchIndex::Ready(SnapshotInfo { memory_index, .. }) => {
                                    memory_index
                                },
                            };

                            if let Some(old_snapshot) = old_snapshot {
                                anyhow::ensure!(old_snapshot.ts <= disk_index_ts);
                            } else {
                                anyhow::ensure!(
                                    memory_index.min_ts()
                                        <= WriteTimestamp::Committed(*disk_index_ts)
                                );
                            }

                            let mut memory_index = memory_index.clone();
                            memory_index.truncate(disk_index_ts.succ()?)?;
                            let snapshot = SnapshotInfo {
                                disk_index: disk_index.clone(),
                                disk_index_ts: *disk_index_ts,
                                disk_index_version: *disk_index_version,
                                memory_index,
                            };

                            let is_next_index_enabled =
                                next_metadata.into_value().config.is_enabled();
                            *entry.get_mut() = if is_next_index_enabled {
                                SearchIndex::Ready(snapshot)
                            } else {
                                SearchIndex::Backfilled(snapshot)
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
                    if metadata.is_search_index() {
                        self.indexes.remove(&deletion.id().internal_id());
                        metrics::log_index_deleted();
                    }
                },
                _ => panic!("Had neither a deletion nor insertion despite checking above"),
            }
        }

        // Handle index updates for our existing search indexes.
        for index in index_registry.search_indexes_by_table(&id.table().table_id) {
            let IndexConfig::Search {
                ref developer_config,
                ..
            } = index.metadata.config
            else {
                continue;
            };
            let tantivy_schema = TantivySearchIndexSchema::new(developer_config);
            let Some(index) = self.indexes.get_mut(&index.id()) else {
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
        self.in_memory_sizes().map(|(_, s)| s).sum()
    }

    pub fn in_memory_sizes(&self) -> impl Iterator<Item = (IndexId, usize)> + '_ {
        self.indexes
            .iter()
            .map(|(id, s)| (*id, s.memory_index().size()))
    }

    pub fn consistency_check(&self) -> anyhow::Result<()> {
        for index in self.indexes.values() {
            index.memory_index().consistency_check()?;
        }
        Ok(())
    }
}
