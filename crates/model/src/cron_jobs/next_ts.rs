use std::time::Duration;

use anyhow::Context;
use chrono::{
    TimeZone,
    Utc,
};
use common::knobs::CRON_SPLAY_SECONDS;
use rand::Rng;
use saffron::Cron;
use sync_types::Timestamp;

use super::types::{
    CronSchedule,
    CronSpec,
};

/// The random delays applied to a cron's scheduled runs. The default is no
/// delay at all, for schedules that don't splay.
#[derive(Default)]
struct Splay {
    /// The delay the previous run had. Subtracting it from a completion time
    /// recovers which occurrence that run belonged to. For a job with no
    /// previous run, the freshly drawn delay counts as already in effect so
    /// the first run can still land in the current window.
    previous_delay: Duration,
    /// The delay the next run gets. This differs from `previous_delay` only
    /// when the previous run had no delay (the job predates splaying) and a
    /// fresh delay was just drawn for it.
    next_delay: Duration,
}

fn cron_splay(
    cron_schedule: &CronSchedule,
    prev_ts: Option<Timestamp>,
    rng: &mut impl Rng,
) -> Splay {
    // `max_splay` bounds the random delay and `period` is the time between
    // the schedule's occurrences. The delay is read back as the previous
    // timestamp's offset within its occurrence, so `max_splay` must never
    // exceed `period`.
    let (max_splay, period) = match cron_schedule {
        // Interval runs are anchored to deploy time, which is already spread
        // out.
        CronSchedule::Interval { .. } => return Splay::default(),
        // With no minute pinned, spread runs across the whole hour.
        CronSchedule::Hourly { minute_utc: None }
        | CronSchedule::Daily {
            minute_utc: None, ..
        }
        | CronSchedule::Weekly {
            minute_utc: None, ..
        }
        | CronSchedule::Monthly {
            minute_utc: None, ..
        } => (Duration::from_secs(60 * 60), Duration::from_secs(60 * 60)),
        // The minute is pinned, so spread runs within it.
        CronSchedule::Hourly {
            minute_utc: Some(_),
        }
        | CronSchedule::Daily {
            minute_utc: Some(_),
            ..
        }
        | CronSchedule::Weekly {
            minute_utc: Some(_),
            ..
        }
        | CronSchedule::Monthly {
            minute_utc: Some(_),
            ..
        }
        | CronSchedule::Cron { .. } => (
            Duration::from_secs(*CRON_SPLAY_SECONDS),
            Duration::from_secs(60),
        ),
    };
    if max_splay.is_zero() {
        return Splay::default();
    }
    // The previous run's timestamp carries the delay and is the only splay
    // state, so a job keeps the same cadence from run to run.
    let previous_delay = prev_ts.map(|prev_ts| {
        let prev_nanos: i64 = prev_ts.into();
        let prev_secs = prev_nanos.div_euclid(1_000_000_000) as u64;
        Duration::from_secs(prev_secs % period.as_secs())
    });
    match previous_delay {
        Some(delay) if !delay.is_zero() => Splay {
            previous_delay: delay,
            next_delay: delay,
        },
        // The previous run had no delay, so the job draws one that starts at
        // the following occurrence.
        Some(_) => Splay {
            previous_delay: Duration::ZERO,
            next_delay: Duration::from_secs(rng.random_range(0..max_splay.as_secs())),
        },
        None => {
            let delay = Duration::from_secs(rng.random_range(0..max_splay.as_secs()));
            Splay {
                previous_delay: delay,
                next_delay: delay,
            }
        },
    }
}

pub fn compute_next_ts(
    cron_spec: &CronSpec,
    prev_ts: Option<Timestamp>,
    now: Timestamp,
    rng: &mut impl Rng,
) -> anyhow::Result<Timestamp> {
    let cron: Cron = match cron_spec.cron_schedule.clone() {
        CronSchedule::Interval { seconds } => {
            let next_ts = match prev_ts {
                Some(prev_ts) => prev_ts.add(Duration::from_secs(seconds as u64))?,
                None => now,
            };
            return Ok(next_ts);
        },
        CronSchedule::Hourly { minute_utc } => format!("{} * * * *", minute_utc.unwrap_or(0))
            .parse()
            .context("Hourly Schedule: Cron parsing from Saffron failed")?,
        CronSchedule::Daily {
            hour_utc,
            minute_utc,
        } => format!("{} {hour_utc} * * *", minute_utc.unwrap_or(0))
            .parse()
            .context("Daily Schedule: Cron parsing from Saffron failed")?,
        CronSchedule::Weekly {
            day_of_week,
            hour_utc,
            minute_utc,
        } => format!("{} {hour_utc} * * {day_of_week}", minute_utc.unwrap_or(0))
            .parse()
            .context("Weekly Schedule: Cron parsing from Saffron failed")?,
        CronSchedule::Monthly {
            day,
            hour_utc,
            minute_utc,
        } => format!("{} {hour_utc} {day} * *", minute_utc.unwrap_or(0))
            .parse()
            .context("Monthly Schedule: Cron parsing from Saffron failed")?,
        CronSchedule::Cron { cron_expr } => cron_expr
            .parse()
            .context("Cron Schedule: Cron parsing from Saffron failed")?,
    };
    // Find the next occurrence on the schedule's own (undelayed) clock, then
    // push it out by the next run's delay. Searching from `now` minus the
    // previous run's delay instead of `now` stops a slow run from skipping an
    // occurrence that its delay pushed past `now`. The result is always after
    // `now` because `next_after` is strictly increasing.
    let Splay {
        previous_delay,
        next_delay,
    } = cron_splay(&cron_spec.cron_schedule, prev_ts, rng);
    let search_after = now.sub(previous_delay).unwrap_or(now);
    let occurrence_utc = cron
        .next_after(Utc.timestamp_nanos(search_after.into()))
        .context("Could not compute next timestamp for cron")?;
    let occurrence: Timestamp = occurrence_utc
        .timestamp_nanos_opt()
        .context("Unable to get nanos from UTC")?
        .try_into()?;
    occurrence.add(next_delay)
}
