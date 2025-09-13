use common::{
    persistence::{
        RepeatablePersistence,
        TimestampRange,
    },
    persistence_helpers::RevisionPair,
    query::Order,
    types::Timestamp,
};
use futures::TryStreamExt;
use futures_async_stream::try_stream;

use crate::{
    bootstrap_model::defaults::BootstrapTableIds,
    committer::table_dependency_sort_key,
};

#[derive(Debug)]
pub struct TransactionRevisions {
    pub ts: Timestamp,
    pub revision_pairs: Vec<RevisionPair>,
}

impl TransactionRevisions {
    pub fn new(
        bootstrap_tables: BootstrapTableIds,
        ts: Timestamp,
        mut revision_pairs: Vec<RevisionPair>,
    ) -> Self {
        assert!(revision_pairs.iter().all(|p| p.ts() == ts));
        // Sort the revision pairs by their commit order.
        revision_pairs
            .sort_by_key(|p| table_dependency_sort_key(bootstrap_tables, p.id, p.document()));
        Self { ts, revision_pairs }
    }
}

#[allow(clippy::needless_lifetimes)]
#[try_stream(ok = TransactionRevisions, error = anyhow::Error)]
pub async fn stream_transactions<'a>(
    bootstrap_tables: BootstrapTableIds,
    reader: &'a RepeatablePersistence,
    // Take in a `range` to ensure that we're getting full transactions and not some split in `(ts,
    // id)` space in the middle of a transaction boundary.
    range: TimestampRange,
    order: Order,
) {
    let revision_stream = reader.load_revision_pairs(None /* tablet_id */, range, order);
    futures::pin_mut!(revision_stream);

    if let Some(first_pair) = revision_stream.try_next().await? {
        let mut curr_ts = first_pair.ts();
        let mut curr_pairs = vec![first_pair];
        while let Some(revision_pair) = revision_stream.try_next().await? {
            if revision_pair.ts() != curr_ts {
                yield TransactionRevisions::new(bootstrap_tables, curr_ts, curr_pairs);
                curr_ts = revision_pair.ts();
                curr_pairs = vec![revision_pair];
            } else {
                curr_pairs.push(revision_pair);
            }
        }
        yield TransactionRevisions::new(bootstrap_tables, curr_ts, curr_pairs);
    }
}
