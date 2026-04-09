use std::time::Duration;

use anyhow::Context;
use chrono::{
    TimeZone,
    Utc,
};
use saffron::Cron;
use sync_types::Timestamp;

use super::types::{
    CronSchedule,
    CronSpec,
};

pub fn compute_next_ts(
    cron_spec: &CronSpec,
    prev_ts: Option<Timestamp>,
    now: Timestamp,
) -> anyhow::Result<Timestamp> {
    let cron: Cron = match cron_spec.cron_schedule.clone() {
        CronSchedule::Interval { seconds } => {
            let next_ts = match prev_ts {
                Some(prev_ts) => prev_ts.add(Duration::from_secs(seconds as u64))?,
                None => now,
            };
            return Ok(next_ts);
        },
        CronSchedule::Hourly { minute_utc } => format!("{minute_utc} * * * *")
            .parse()
            .context("Hourly Schedule: Cron parsing from Saffron failed")?,
        CronSchedule::Daily {
            hour_utc,
            minute_utc,
        } => format!("{minute_utc} {hour_utc} * * *")
            .parse()
            .context("Daily Schedule: Cron parsing from Saffron failed")?,
        CronSchedule::Weekly {
            day_of_week,
            hour_utc,
            minute_utc,
        } => format!("{minute_utc} {hour_utc} * * {day_of_week}")
            .parse()
            .context("Weekly Schedule: Cron parsing from Saffron failed")?,
        CronSchedule::Monthly {
            day,
            hour_utc,
            minute_utc,
        } => format!("{minute_utc} {hour_utc} {day} * *")
            .parse()
            .context("Monthly Schedule: Cron parsing from Saffron failed")?,
        CronSchedule::Cron { cron_expr } => cron_expr
            .parse()
            .context("Cron Schedule: Cron parsing from Saffron failed")?,
    };
    let now_nanos: i64 = now.into();
    let now_utc = Utc.timestamp_nanos(now_nanos);
    let next_ts_utc = match cron.next_after(now_utc) {
        Some(next_ts_utc) => next_ts_utc,
        None => return Err(anyhow::anyhow!("Could not compute next timestamp for cron")),
    };
    let next_ts_nanos = next_ts_utc
        .timestamp_nanos_opt()
        .context("Unable to get nanos from UTC")?;
    let next_ts: Timestamp = next_ts_nanos.try_into()?;
    Ok(next_ts)
}
