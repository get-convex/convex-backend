use std::sync::Arc;

use chrono::{
    TimeZone,
    Utc,
};
use common::{
    document::ParsedDocument,
    runtime::Runtime,
};
use database::{
    SystemMetadataModel,
    Transaction,
};
use saffron::Cron;
use sync_types::Timestamp;
use value::{
    TableName,
    TableNamespace,
};

use crate::{
    periodic_backup::types::PeriodicBackupConfig,
    SystemIndex,
    SystemTable,
};

pub mod types;

pub const PERIODIC_BACKUP_CONFIG_TABLE: TableName = TableName::const_new("_periodic_backup_config");

pub struct PeriodicBackupConfigTable;
impl SystemTable for PeriodicBackupConfigTable {
    type Metadata = PeriodicBackupConfig;

    const TABLE_NAME: TableName = PERIODIC_BACKUP_CONFIG_TABLE;

    fn indexes() -> Vec<SystemIndex<Self>> {
        vec![]
    }
}

pub struct PeriodicBackupModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> PeriodicBackupModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    pub async fn get(
        &mut self,
    ) -> anyhow::Result<Option<Arc<ParsedDocument<PeriodicBackupConfig>>>> {
        self.tx
            .query_system(
                TableNamespace::Global,
                &SystemIndex::<PeriodicBackupConfigTable>::by_id(),
            )?
            .unique()
            .await
    }

    /// Insert or update the singleton config row. `cronspec` is validated
    /// with `saffron::Cron`; an invalid expression is a 400 at the HTTP layer.
    /// `next_run_ts` is recomputed from the cronspec relative to "now".
    pub async fn set(
        &mut self,
        cronspec: String,
        include_storage: bool,
    ) -> anyhow::Result<PeriodicBackupConfig> {
        let cron: Cron = cronspec
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid cron expression: {e}"))?;
        anyhow::ensure!(
            cron.any(),
            "cron expression {cronspec} matches no future time"
        );

        let now_ts = self.tx.runtime().generate_timestamp()?;
        let next_run_ts = next_run_ts_from(&cron, now_ts)?;

        let existing = self.get().await?;
        // Preserve last_run_ts across config updates so consecutive edits
        // don't lose the "last fired" history.
        let last_run_ts = existing.as_ref().and_then(|doc| (***doc).last_run_ts);

        let new = PeriodicBackupConfig {
            cronspec,
            include_storage,
            next_run_ts,
            last_run_ts,
        };

        match existing {
            Some(doc) => {
                SystemMetadataModel::new_global(self.tx)
                    .replace(doc.id().to_owned(), new.clone().try_into()?)
                    .await?;
            },
            None => {
                SystemMetadataModel::new_global(self.tx)
                    .insert(&PERIODIC_BACKUP_CONFIG_TABLE, new.clone().try_into()?)
                    .await?;
            },
        }
        Ok(new)
    }

    /// Remove the singleton row entirely. The worker stops triggering
    /// exports until a new config is created.
    pub async fn disable(&mut self) -> anyhow::Result<bool> {
        if let Some(doc) = self.get().await? {
            SystemMetadataModel::new_global(self.tx)
                .delete(doc.id().to_owned())
                .await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// After firing, advance `next_run_ts` and stamp `last_run_ts`. Caller
    /// must already hold the row (lookup is performed inside).
    pub async fn record_run(
        &mut self,
        run_ts: Timestamp,
    ) -> anyhow::Result<Option<PeriodicBackupConfig>> {
        let Some(doc) = self.get().await? else {
            return Ok(None);
        };
        let cron: Cron = doc.cronspec.parse()?;
        let next_run_ts = next_run_ts_from(&cron, run_ts)?;
        let id = doc.id().to_owned();
        let mut updated = (*doc).clone().into_value();
        updated.last_run_ts = Some(run_ts);
        updated.next_run_ts = next_run_ts;
        SystemMetadataModel::new_global(self.tx)
            .replace(id, updated.clone().try_into()?)
            .await?;
        Ok(Some(updated))
    }
}

/// Compute the next match of `cron` strictly after the given Convex
/// `Timestamp`, returning a Convex `Timestamp` again.
pub fn next_run_ts_from(cron: &Cron, after: Timestamp) -> anyhow::Result<Timestamp> {
    let after_nanos: i64 = after.into();
    let after_dt = Utc
        .timestamp_nanos(after_nanos)
        // saffron expects strictly greater-than; nudge by 1 second so
        // we don't immediately re-fire on the cron's current minute.
        .checked_add_signed(chrono::Duration::seconds(1))
        .context_else("timestamp arithmetic overflow")?;
    let next = cron
        .next_after(after_dt)
        .context_else("cron has no future occurrences")?;
    let next_nanos = next
        .timestamp_nanos_opt()
        .context_else("cron next time outside i64 nanos range")?;
    Timestamp::try_from(next_nanos)
}

trait OptionExt<T> {
    fn context_else(self, msg: &'static str) -> anyhow::Result<T>;
}
impl<T> OptionExt<T> for Option<T> {
    fn context_else(self, msg: &'static str) -> anyhow::Result<T> {
        self.ok_or_else(|| anyhow::anyhow!(msg))
    }
}

#[cfg(test)]
mod tests {
    use sync_types::Timestamp;
    use value::ConvexObject;

    use super::types::PeriodicBackupConfig;

    #[test]
    fn round_trip_without_last_run() -> anyhow::Result<()> {
        let original = PeriodicBackupConfig {
            cronspec: "0 3 * * *".to_string(),
            include_storage: false,
            next_run_ts: Timestamp::try_from(1_700_000_000_000_000_000u64)?,
            last_run_ts: None,
        };
        let obj: ConvexObject = original.clone().try_into()?;
        let decoded: PeriodicBackupConfig = obj.try_into()?;
        assert_eq!(original, decoded);
        Ok(())
    }

    #[test]
    fn round_trip_with_last_run() -> anyhow::Result<()> {
        let original = PeriodicBackupConfig {
            cronspec: "30 7 * * 1".to_string(),
            include_storage: true,
            next_run_ts: Timestamp::try_from(1_700_000_000_000_000_000u64)?,
            last_run_ts: Some(Timestamp::try_from(1_699_900_000_000_000_000u64)?),
        };
        let obj: ConvexObject = original.clone().try_into()?;
        let decoded: PeriodicBackupConfig = obj.try_into()?;
        assert_eq!(original, decoded);
        Ok(())
    }
}
