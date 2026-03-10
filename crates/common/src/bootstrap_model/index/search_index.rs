use sync_types::Timestamp;
use value::InternalId;

/// Generic backfill state for search indexes (text and vector).
/// Parameterized by the segment type `S`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct BackfillState<S> {
    pub segments: Vec<S>,
    /// None at the start of the backfill, set after the first iteration of the
    /// backfill.
    pub cursor: Option<SearchBackfillCursor>,
    pub staged: bool,
}

/// There are two formats for `BackfillCursor`, depending on the algorithm used
/// to backfill. We can collapse this enum when we've migrated successfully to
/// `WalkingForwards`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum SearchBackfillCursor {
    /// Backfilling cursor when iterating over a single snapshot.
    AtSnapshot {
        backfill_snapshot_ts: Timestamp,
        /// Last document id read in the most recent backfill iteration.
        cursor: InternalId,
    },
    /// Backfilling cursor for the algorithm with two phases: walking a section
    /// of the table at a recent snapshot and then walking the document log
    /// since the last segment was written, filling in the updates.
    WalkingForwards {
        /// The timestamp of the last segment that was backfilled. We have to
        /// scan the document log for changes after this timestamp to documents
        /// that have already been backfilled.
        last_segment_ts: Timestamp,
        /// The last document by_id key we indexed in the last segment
        /// backfilled. We start the table scan after this cursor for the next
        /// segment.
        table_scan_cursor: Vec<u8>,
    },
}

impl SearchBackfillCursor {
    pub fn backfill_ts(&self) -> Timestamp {
        match self {
            SearchBackfillCursor::AtSnapshot {
                backfill_snapshot_ts,
                ..
            } => *backfill_snapshot_ts,
            SearchBackfillCursor::WalkingForwards {
                last_segment_ts, ..
            } => *last_segment_ts,
        }
    }
}

impl<S> BackfillState<S> {
    pub fn new(staged: bool) -> Self {
        Self {
            segments: vec![],
            cursor: None,
            staged,
        }
    }

    pub fn backfill_ts(&self) -> Option<Timestamp> {
        self.cursor.as_ref().map(|c| c.backfill_ts())
    }
}
