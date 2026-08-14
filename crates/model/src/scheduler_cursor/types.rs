use common::types::Timestamp;
use serde::{
    Deserialize,
    Serialize,
};
use value::codegen_convex_serialization;

/// Checkpointed position of the scheduled job executor, so a restart resumes
/// from a bounded point in `_scheduled_jobs.by_next_ts` instead of scanning
/// the whole index from `null`.
///
/// There is one cursor document per component namespace, matching the
/// `_scheduled_jobs` table it tracks: indexes are per-tablet, so the executor
/// walks one `by_next_ts` per namespace and each advances independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerCursor {
    /// Jobs with `scheduled_ts` below this have been started, so `by_next_ts`
    /// index is scanned from here. Advancing it is what skips the
    /// key-change tombstones left behind by completed jobs.
    pub last_scheduled_ts: Timestamp,
    /// Commit timestamp up to which the executor has scanned the table's
    /// document log, in commit order, to catch jobs whose `next_ts` was
    /// already below `last_scheduled_ts` by the time their write committed.
    /// Bounded below `last_scheduled_ts` so the scan cannot silently fall
    /// behind the jobs it is meant to catch.
    pub last_commit_ts: Timestamp,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedSchedulerCursor {
    last_scheduled_ts: i64,
    last_commit_ts: i64,
}

impl TryFrom<SchedulerCursor> for SerializedSchedulerCursor {
    type Error = anyhow::Error;

    fn try_from(value: SchedulerCursor) -> anyhow::Result<Self> {
        Ok(Self {
            last_scheduled_ts: value.last_scheduled_ts.into(),
            last_commit_ts: value.last_commit_ts.into(),
        })
    }
}

impl TryFrom<SerializedSchedulerCursor> for SchedulerCursor {
    type Error = anyhow::Error;

    fn try_from(value: SerializedSchedulerCursor) -> anyhow::Result<Self> {
        Ok(Self {
            last_scheduled_ts: value.last_scheduled_ts.try_into()?,
            last_commit_ts: value.last_commit_ts.try_into()?,
        })
    }
}

codegen_convex_serialization!(SchedulerCursor, SerializedSchedulerCursor);
