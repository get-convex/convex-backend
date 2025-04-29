use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    iter,
    ops::Bound,
    sync::Arc,
};

use async_trait::async_trait;
use cmd_util::env::config_test;
use futures::{
    stream,
    StreamExt,
};
use itertools::Itertools;
use parking_lot::Mutex;
use serde_json::Value as JsonValue;
use value::{
    InternalDocumentId,
    ResolvedDocumentId,
    TabletId,
};

#[cfg(test)]
use super::persistence_test_suite;
use crate::{
    document::ResolvedDocument,
    index::{
        IndexEntry,
        IndexKeyBytes,
    },
    interval::{
        End,
        Interval,
        StartIncluded,
    },
    persistence::{
        ConflictStrategy,
        DocumentLogEntry,
        DocumentPrevTsQuery,
        DocumentStream,
        IndexStream,
        LatestDocument,
        Persistence,
        PersistenceGlobalKey,
        PersistenceReader,
        RetentionValidator,
        TimestampRange,
    },
    query::Order,
    run_persistence_test_suite,
    types::{
        DatabaseIndexUpdate,
        DatabaseIndexValue,
        IndexId,
        PersistenceVersion,
        Timestamp,
    },
};

#[derive(Clone)]
pub struct TestPersistence {
    inner: Arc<Mutex<Inner>>,
}

impl TestPersistence {
    pub fn new() -> Self {
        config_test();
        let inner = Inner {
            is_fresh: true,
            is_read_only: false,
            log: BTreeMap::new(),
            index: BTreeMap::new(),
            persistence_globals: BTreeMap::new(),
        };
        Self::new_inner(Arc::new(Mutex::new(inner)), false).unwrap()
    }

    /// Pass in an Inner to store state across TestPersistence instances.
    fn new_inner(inner: Arc<Mutex<Inner>>, allow_read_only: bool) -> anyhow::Result<Self> {
        anyhow::ensure!(allow_read_only || !inner.lock().is_read_only);
        Ok(Self { inner })
    }
}

#[async_trait]
impl Persistence for TestPersistence {
    fn is_fresh(&self) -> bool {
        self.inner.lock().is_fresh
    }

    fn reader(&self) -> Arc<dyn PersistenceReader> {
        Arc::new(self.clone())
    }

    async fn write(
        &self,
        documents: Vec<DocumentLogEntry>,
        indexes: BTreeSet<(Timestamp, DatabaseIndexUpdate)>,
        conflict_strategy: ConflictStrategy,
    ) -> anyhow::Result<()> {
        let mut inner = self.inner.lock();
        for update in documents {
            anyhow::ensure!(
                conflict_strategy == ConflictStrategy::Overwrite
                    || !inner.log.contains_key(&(update.ts, update.id)),
                "Unique constraint not satisifed. Failed to write document at ts {} with id {}: \
                 (document, ts) pair already exists",
                update.ts,
                update.id
            );
            inner
                .log
                .insert((update.ts, update.id), (update.value, update.prev_ts));
        }
        inner.is_fresh = false;
        for (ts, update) in indexes {
            let index_key_bytes = update.key.to_bytes();
            anyhow::ensure!(
                conflict_strategy == ConflictStrategy::Overwrite
                    || !inner
                        .index
                        .get(&update.index_id)
                        .map(|idx| idx.contains_key(&(index_key_bytes.clone(), ts)))
                        .unwrap_or(false),
                "Unique constraint not satisfied. Failed to write to index {} at ts {} with key \
                 {:?}: (key, ts) pair already exists",
                update.index_id,
                ts,
                update.key
            );
            inner
                .index
                .entry(update.index_id)
                .or_default()
                .insert((index_key_bytes, ts), update.value);
        }
        Ok(())
    }

    async fn set_read_only(&self, read_only: bool) -> anyhow::Result<()> {
        self.inner.lock().is_read_only = read_only;
        Ok(())
    }

    async fn write_persistence_global(
        &self,
        key: PersistenceGlobalKey,
        value: JsonValue,
    ) -> anyhow::Result<()> {
        self.inner.lock().persistence_globals.insert(key, value);
        Ok(())
    }

    async fn load_index_chunk(
        &self,
        cursor: Option<IndexEntry>,
        chunk_size: usize,
    ) -> anyhow::Result<Vec<IndexEntry>> {
        let mut inner = self.inner.lock();
        let index = &mut inner.index;
        let index_entries = index
            .iter()
            .flat_map(|(index_id, tree)| {
                tree.iter().map(|((key, ts), v)| IndexEntry {
                    index_id: *index_id,
                    deleted: v.is_delete(),
                    key_prefix: key.0.clone(),
                    key_suffix: None,
                    key_sha256: key.0.clone(),
                    ts: *ts,
                })
            })
            .filter(|index_entry| match cursor {
                None => true,
                Some(ref cursor) => index_entry > cursor,
            })
            .take(chunk_size)
            .collect();
        Ok(index_entries)
    }

    async fn delete_index_entries(&self, expired_rows: Vec<IndexEntry>) -> anyhow::Result<usize> {
        let mut inner = self.inner.lock();
        let index = &mut inner.index;
        let mut total_deleted = 0;
        for expired_row in expired_rows {
            if index
                .get_mut(&expired_row.index_id)
                .unwrap()
                .remove(&(IndexKeyBytes(expired_row.key_prefix), expired_row.ts))
                .is_some()
            {
                total_deleted += 1;
            }
        }
        Ok(total_deleted)
    }

    async fn delete(
        &self,
        documents: Vec<(Timestamp, InternalDocumentId)>,
    ) -> anyhow::Result<usize> {
        let mut inner = self.inner.lock();
        let log = &mut inner.log;
        let mut total_deleted = 0;
        for expired_doc in documents {
            if log.remove(&expired_doc).is_some() {
                total_deleted += 1;
            }
        }
        Ok(total_deleted)
    }
}

#[async_trait]
impl PersistenceReader for TestPersistence {
    fn load_documents(
        &self,
        range: TimestampRange,
        order: Order,
        _page_size: u32,
        _retention_validator: Arc<dyn RetentionValidator>,
    ) -> DocumentStream<'_> {
        let log = { self.inner.lock().log.clone() };

        let iter = log
            .into_iter()
            .map(|((ts, id), (value, prev_ts))| DocumentLogEntry {
                ts,
                id,
                value,
                prev_ts,
            })
            .filter(move |entry| range.contains(entry.ts))
            // Mimic the sort in Postgres that is by internal id.
            .sorted_by_key(|entry| (entry.ts, entry.id.internal_id()))
            .map(Ok);
        match order {
            Order::Asc => stream::iter(iter).boxed(),
            Order::Desc => stream::iter(iter.rev()).boxed(),
        }
    }

    async fn previous_revisions(
        &self,
        ids: BTreeSet<(InternalDocumentId, Timestamp)>,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> anyhow::Result<BTreeMap<(InternalDocumentId, Timestamp), DocumentLogEntry>> {
        let min_ts = ids.iter().map(|(_, ts)| *ts).min();
        let result = {
            let inner = self.inner.lock();
            let result = ids
                .into_iter()
                .filter_map(|(id, ts)| {
                    inner
                        .log
                        .iter()
                        .filter(|((log_ts, log_id), _)| log_id == &id && log_ts < &ts)
                        .max_by_key(|(log_ts, _)| *log_ts)
                        .map(|((log_ts, _), (doc, prev_ts))| {
                            (
                                (id, ts),
                                DocumentLogEntry {
                                    id,
                                    ts: *log_ts,
                                    value: doc.clone(),
                                    prev_ts: *prev_ts,
                                },
                            )
                        })
                })
                .collect();
            result
        };
        if let Some(min_ts) = min_ts {
            retention_validator
                .validate_document_snapshot(min_ts)
                .await?;
        }
        Ok(result)
    }

    async fn previous_revisions_of_documents(
        &self,
        ids: BTreeSet<DocumentPrevTsQuery>,
        retention_validator: Arc<dyn RetentionValidator>,
    ) -> anyhow::Result<BTreeMap<DocumentPrevTsQuery, DocumentLogEntry>> {
        let min_ts = ids.iter().map(|DocumentPrevTsQuery { ts, .. }| *ts).min();
        let result = {
            let inner = self.inner.lock();
            let result = ids
                .into_iter()
                .filter_map(|DocumentPrevTsQuery { id, ts, prev_ts }| {
                    inner.log.get(&(prev_ts, id)).map(|(doc, prev_prev_ts)| {
                        (
                            DocumentPrevTsQuery { id, ts, prev_ts },
                            DocumentLogEntry {
                                id,
                                ts: prev_ts,
                                value: doc.clone(),
                                prev_ts: *prev_prev_ts,
                            },
                        )
                    })
                })
                .collect();
            result
        };
        if let Some(min_ts) = min_ts {
            retention_validator
                .validate_document_snapshot(min_ts)
                .await?;
        }
        Ok(result)
    }

    fn index_scan(
        &self,
        index_id: IndexId,
        _tablet_id: TabletId,
        read_timestamp: Timestamp,
        interval: &Interval,
        order: Order,
        _size_hint: usize,
        _retention_validator: Arc<dyn RetentionValidator>,
    ) -> IndexStream<'_> {
        let interval = interval.clone();
        // Add timestamp.
        let lower = match interval.start {
            StartIncluded(v) => Bound::Included((v.into(), Timestamp::MIN)),
        };
        let upper = match interval.end {
            End::Excluded(v) => Bound::Excluded((v.into(), Timestamp::MIN)),
            End::Unbounded => Bound::Unbounded,
        };

        let lock = self.inner.lock();
        let index = lock.index.get(&index_id);

        // BTreeMap is not happy if you give it an empty range. Copy how it detects
        // the range is empty and a void calling it.
        let index = match (&lower, &upper) {
            (Bound::Excluded(s), Bound::Excluded(e)) if s == e => None,
            (Bound::Included(s) | Bound::Excluded(s), Bound::Included(e) | Bound::Excluded(e))
                if s > e =>
            {
                None
            },
            _ => index,
        };

        let it: Box<dyn Iterator<Item = _> + Send> = match index {
            Some(index) => {
                let it = index.range((lower, upper));
                match order {
                    Order::Asc => Box::new(it),
                    Order::Desc => Box::new(it.rev()),
                }
            },
            None => Box::new(iter::empty()),
        };

        let mut results: Vec<(IndexKeyBytes, Timestamp, ResolvedDocumentId)> = Vec::new();
        let mut maybe_add_value =
            |entry: Option<(&(IndexKeyBytes, Timestamp), &DatabaseIndexValue)>| match entry {
                Some(((k, ts), value)) => match value {
                    DatabaseIndexValue::Deleted => {},
                    DatabaseIndexValue::NonClustered(doc_id) => {
                        // Lookup the document by id and timestamp.
                        results.push((k.clone(), *ts, *doc_id));
                    },
                },
                None => {},
            };
        let mut previous: Option<(&(IndexKeyBytes, Timestamp), &DatabaseIndexValue)> = None;
        for current in it {
            if current.0 .1 > read_timestamp {
                // Outside of read snapshot.
                continue;
            }
            let different = match previous {
                Some(previous) => previous.0 .0 != current.0 .0,
                None => true,
            };
            if different {
                match order {
                    Order::Asc => maybe_add_value(previous),
                    Order::Desc => maybe_add_value(Some(current)),
                };
            }
            previous = Some(current);
        }
        // Yield the last value if applicable.
        match order {
            Order::Asc => maybe_add_value(previous),
            Order::Desc => {},
        };

        let results: Vec<anyhow::Result<(IndexKeyBytes, LatestDocument)>> = results
            .into_iter()
            .map(|(k, ts, doc_id)| -> anyhow::Result<_> {
                let (value, prev_ts) = lock.lookup(doc_id.into(), ts)?;
                Ok((k, LatestDocument { ts, value, prev_ts }))
            })
            .collect();

        stream::iter(results).boxed()
    }

    async fn get_persistence_global(
        &self,
        key: PersistenceGlobalKey,
    ) -> anyhow::Result<Option<JsonValue>> {
        Ok(self.inner.lock().persistence_globals.get(&key).cloned())
    }

    fn version(&self) -> PersistenceVersion {
        PersistenceVersion::default()
    }
}

struct Inner {
    is_fresh: bool,
    is_read_only: bool,
    log: BTreeMap<(Timestamp, InternalDocumentId), (Option<ResolvedDocument>, Option<Timestamp>)>,
    index: BTreeMap<IndexId, BTreeMap<(IndexKeyBytes, Timestamp), DatabaseIndexValue>>,
    persistence_globals: BTreeMap<PersistenceGlobalKey, JsonValue>,
}

impl Inner {
    // Lookup object by (id, timestamp). The document must exist.
    fn lookup(
        &self,
        doc_id: InternalDocumentId,
        ts: Timestamp,
    ) -> anyhow::Result<(ResolvedDocument, Option<Timestamp>)> {
        let (value, prev_ts) = self
            .log
            .get(&(ts, doc_id))
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Dangling index reference"))?;
        Ok((
            value.ok_or_else(|| anyhow::anyhow!("Index reference to deleted document"))?,
            prev_ts,
        ))
    }
}

run_persistence_test_suite!(
    db,
    Arc::new(Mutex::new(Inner {
        is_fresh: true,
        is_read_only: false,
        log: BTreeMap::new(),
        index: BTreeMap::new(),
        persistence_globals: BTreeMap::new(),
    })),
    TestPersistence::new_inner(db.clone(), false)?,
    TestPersistence::new_inner(db.clone(), true)?
);
