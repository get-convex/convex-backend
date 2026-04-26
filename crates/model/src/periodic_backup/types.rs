use serde::{
    Deserialize,
    Serialize,
};
use sync_types::Timestamp;
use value::codegen_convex_serialization;

/// Configuration for self-hosted periodic snapshot exports. The
/// `_periodic_backup_config` table holds at most one row; when present,
/// the periodic-backup background worker triggers a new snapshot export
/// (via the same path as a manual "Back up now") whenever
/// `next_run_ts <= now`, then advances `next_run_ts` to the next match
/// of `cronspec` and updates `last_run_ts`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeriodicBackupConfig {
    /// 5-field UTC cron expression (e.g. `0 3 * * *` for daily 03:00 UTC).
    /// Validated with `saffron::Cron` at write time.
    pub cronspec: String,
    /// Whether triggered exports include file storage in the produced zip.
    pub include_storage: bool,
    /// Wall-clock time of the next scheduled run, recomputed each tick from
    /// `cronspec`. Stored as a `Timestamp` so it shares the persistence path
    /// the rest of the system uses.
    pub next_run_ts: Timestamp,
    /// Wall-clock time of the most recent successful trigger; `None` until
    /// the worker fires for the first time.
    pub last_run_ts: Option<Timestamp>,
}

/// Snake-case field names — this struct is serialized via
/// `codegen_convex_serialization!` and ends up as the actual stored
/// `_periodic_backup_config` row. The dashboard reads the row through
/// `udfs.periodicBackup.get`, so the field names here must match what
/// `npm-packages/system-udfs/convex/schema.ts` declares for the table.
///
/// The camelCase aliases hydrate legacy rows written before this
/// struct dropped its `rename_all = "camelCase"` attribute — without
/// them, the next `set()` fails its existing-row fetch and locks the
/// user out of saving. Writes always use snake_case, so stale rows
/// self-heal on the next save.
#[derive(Serialize, Deserialize)]
pub struct SerializedPeriodicBackupConfig {
    cronspec: String,
    #[serde(alias = "includeStorage")]
    include_storage: bool,
    #[serde(alias = "nextRunTs")]
    next_run_ts: i64,
    #[serde(default, alias = "lastRunTs")]
    last_run_ts: Option<i64>,
}

impl TryFrom<PeriodicBackupConfig> for SerializedPeriodicBackupConfig {
    type Error = anyhow::Error;

    fn try_from(value: PeriodicBackupConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            cronspec: value.cronspec,
            include_storage: value.include_storage,
            next_run_ts: value.next_run_ts.into(),
            last_run_ts: value.last_run_ts.map(|t| t.into()),
        })
    }
}

impl TryFrom<SerializedPeriodicBackupConfig> for PeriodicBackupConfig {
    type Error = anyhow::Error;

    fn try_from(value: SerializedPeriodicBackupConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            cronspec: value.cronspec,
            include_storage: value.include_storage,
            next_run_ts: Timestamp::try_from(value.next_run_ts)?,
            last_run_ts: value.last_run_ts.map(Timestamp::try_from).transpose()?,
        })
    }
}

codegen_convex_serialization!(PeriodicBackupConfig, SerializedPeriodicBackupConfig);
