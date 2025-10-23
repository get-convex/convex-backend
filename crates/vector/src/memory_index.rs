use std::{
    collections::BTreeSet,
    mem,
};

use common::types::{
    Timestamp,
    WriteTimestamp,
};
use imbl::{
    OrdMap,
    OrdSet,
    Vector,
};
use qdrant_segment::spaces::{
    metric::Metric,
    simple::CosineMetric,
};
use value::InternalId;

use crate::{
    qdrant_index::{
        NormalizedQdrantDocument,
        QdrantDocument,
    },
    query::{
        CompiledVectorFilter,
        CompiledVectorSearch,
        VectorSearchQueryResult,
    },
};

#[derive(Clone)]
pub struct MemoryVectorIndex {
    min_ts: WriteTimestamp,
    max_ts: WriteTimestamp,

    documents: OrdMap<InternalId, Revision>,
    documents_size: usize,

    tombstones: Vector<(WriteTimestamp, NormalizedQdrantDocument)>,
    tombstones_size: usize,

    transactions: OrdSet<WriteTimestamp>,
}

impl MemoryVectorIndex {
    pub fn new(base_ts: WriteTimestamp) -> Self {
        Self {
            min_ts: base_ts,
            max_ts: base_ts,

            documents: OrdMap::new(),
            documents_size: 0,

            tombstones: Vector::new(),
            tombstones_size: 0,

            transactions: OrdSet::new(),
        }
    }

    pub fn size(&self) -> usize {
        let mut size = 0;

        size += self.documents.len() * mem::size_of::<(InternalId, Revision)>();
        size += self.documents_size;

        size +=
            self.tombstones.len() * mem::size_of::<(WriteTimestamp, NormalizedQdrantDocument)>();
        size += self.tombstones_size;

        size += self.transactions.len() * mem::size_of::<WriteTimestamp>();

        size
    }

    pub fn min_ts(&self) -> WriteTimestamp {
        self.min_ts
    }

    pub fn num_transactions(&self) -> usize {
        self.transactions.len()
    }

    pub fn update(
        &mut self,
        id: InternalId,
        ts: WriteTimestamp,
        old_value: Option<QdrantDocument>,
        new_value: Option<QdrantDocument>,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.min_ts <= ts,
            "Expected min_ts:{:?} <= ts:{ts:?} ",
            self.min_ts
        );
        anyhow::ensure!(
            self.max_ts <= ts,
            "Expected max_ts:{:?} <= ts:{ts:?} ",
            self.max_ts
        );
        self.max_ts = ts;
        {
            if !self.transactions.contains(&ts) {
                if let Some(prev_ts) = self.transactions.get_max() {
                    anyhow::ensure!(*prev_ts < ts);
                }
                self.transactions.insert(ts);
            }
        }
        if let Some(old_value) = old_value {
            let normalized = NormalizedQdrantDocument::from(old_value);
            self.tombstones_size += normalized.size();
            self.tombstones.push_back((ts, normalized));
        }
        if self.documents.contains_key(&id) {
            let old_value = self.documents.remove(&id).unwrap();
            self.documents_size -= old_value.document.size();
        }
        if let Some(new_value) = new_value {
            let normalized = NormalizedQdrantDocument::from(new_value);
            self.documents_size += normalized.size();
            let revision = Revision {
                ts,
                document: normalized,
            };
            self.documents.insert(id, revision);
        }
        Ok(())
    }

    pub fn truncate(&mut self, new_min_ts: Timestamp) -> anyhow::Result<()> {
        let new_min_ts = WriteTimestamp::Committed(new_min_ts);
        anyhow::ensure!(
            new_min_ts >= self.min_ts,
            "Expected new_min_ts:{new_min_ts:?} >= min_ts:{:?} ",
            self.min_ts
        );
        let to_remove = self
            .documents
            .iter()
            .filter(|(_, document)| document.ts < new_min_ts)
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();

        for id in to_remove {
            let revision = self.documents.remove(&id).unwrap();
            self.documents_size -= revision.document.size();
        }

        while let Some((ts, _)) = self.tombstones.front()
            && *ts < new_min_ts
        {
            let (_, tombstone) = self.tombstones.pop_front().unwrap();
            self.tombstones_size -= tombstone.size();
        }

        while let Some(ts) = self.transactions.get_min()
            && *ts < new_min_ts
        {
            let ts = *ts;
            self.transactions.remove(&ts);
        }

        self.min_ts = new_min_ts;
        self.min_ts = new_min_ts;
        self.max_ts = self.max_ts.max(new_min_ts);

        Ok(())
    }

    pub fn updated_matches(
        &self,
        snapshot_ts: Timestamp,
        query: &CompiledVectorSearch,
    ) -> anyhow::Result<BTreeSet<InternalId>> {
        anyhow::ensure!(
            self.min_ts <= WriteTimestamp::Committed(snapshot_ts.succ()?),
            "Timestamps are out of order! min ts:{:?} snapshot_ts:{snapshot_ts}",
            self.min_ts,
        );
        let mut updated = BTreeSet::new();
        for (ts, document) in &self.tombstones {
            if *ts <= WriteTimestamp::Committed(snapshot_ts) {
                continue;
            }
            if document.matches(query) {
                updated.insert(document.internal_id);
            }
        }
        Ok(updated)
    }

    pub fn query(
        &self,
        snapshot_ts: Timestamp,
        query: &CompiledVectorSearch,
    ) -> anyhow::Result<Vec<VectorSearchQueryResult>> {
        anyhow::ensure!(
            self.min_ts <= WriteTimestamp::Committed(snapshot_ts.succ()?),
            "Timestamps are out of order!  min ts:{:?} snapshot_ts:{snapshot_ts}",
            self.min_ts,
        );
        let query_vector = Vec::from(query.vector.clone());
        let query_vector = CosineMetric::preprocess(query_vector);
        let mut candidates = vec![];

        for (&id, revision) in &self.documents {
            if revision.document.matches(query) {
                let distance = CosineMetric::similarity(&query_vector, &revision.document.vector);
                candidates.push(VectorSearchQueryResult {
                    score: distance,
                    id,
                    ts: revision.ts,
                });
            }
        }

        candidates.sort_by(|a, b| a.cmp(b).reverse());
        candidates.truncate(query.limit as usize);

        Ok(candidates)
    }
}

#[derive(Clone)]
pub struct Revision {
    ts: WriteTimestamp,
    document: NormalizedQdrantDocument,
}

impl NormalizedQdrantDocument {
    fn matches(&self, query: &CompiledVectorSearch) -> bool {
        if query.filter_conditions.is_empty() {
            return true;
        }
        for (field_path, filter_condition) in &query.filter_conditions {
            let Some(value) = self.filter_fields.get(field_path) else {
                return false;
            };
            let condition_result = match filter_condition {
                CompiledVectorFilter::Eq(term) => term == value,
                CompiledVectorFilter::In(terms) => terms.iter().any(|t| t == value),
            };
            if condition_result {
                return true;
            }
        }
        false
    }
}
