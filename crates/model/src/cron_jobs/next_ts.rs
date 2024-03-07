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
    let prev_ts = prev_ts.unwrap_or(now);
    let prev_ts_nanos: i64 = prev_ts.try_into()?;
    let prev_ts_utc = Utc.timestamp_nanos(prev_ts_nanos);
    let next_ts_utc = match cron.next_after(prev_ts_utc) {
        Some(next_ts_utc) => next_ts_utc,
        None => return Err(anyhow::anyhow!("Could not compute next timestamp for cron")),
    };
    let next_ts_nanos = next_ts_utc
        .timestamp_nanos_opt()
        .context("Unable to get nanos from UTC")?;
    let next_ts: Timestamp = next_ts_nanos.try_into()?;
    Ok(next_ts)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use sync_types::{
        Timestamp,
        UdfPath,
    };
    use value::ConvexArray;

    use crate::cron_jobs::{
        next_ts::compute_next_ts,
        types::{
            CronSchedule,
            CronSpec,
        },
    };

    #[test]
    fn test_compute_next_ts_interval() {
        // Every minute
        let cron_spec = CronSpec {
            udf_path: UdfPath::from_str("test").unwrap().canonicalize(),
            udf_args: ConvexArray::try_from(vec![]).unwrap(),
            cron_schedule: CronSchedule::Interval { seconds: 60 },
        };

        // Mar 01 2023 08:35:00 UTC
        let now = Timestamp::try_from(i64::pow(10, 9) * 1677659700).unwrap();
        let mut prev_ts = None;
        let mut result = compute_next_ts(&cron_spec, prev_ts, now);
        assert_eq!(result.unwrap(), now);

        prev_ts = Some(now);
        result = compute_next_ts(&cron_spec, prev_ts, now);
        // Mar 01 2023 08:36:00 UTC
        let expected = Timestamp::try_from(i64::pow(10, 9) * 1677659760).unwrap();
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_compute_next_ts_hourly() {
        // Every hour on the 5th minute
        let cron_spec = CronSpec {
            udf_path: UdfPath::from_str("test").unwrap().canonicalize(),
            udf_args: ConvexArray::try_from(vec![]).unwrap(),
            cron_schedule: CronSchedule::Hourly { minute_utc: 5 },
        };

        // Mar 01 2023 08:35:00 UTC
        let now = Timestamp::try_from(i64::pow(10, 9) * 1677659700).unwrap();
        let mut prev_ts = None;
        let mut result = compute_next_ts(&cron_spec, prev_ts, now);
        // Mar 01 2023 09:05:00 UTC
        let mut expected = Timestamp::try_from(i64::pow(10, 9) * 1677661500).unwrap();
        assert_eq!(result.unwrap(), expected);

        prev_ts = Some(expected);
        result = compute_next_ts(&cron_spec, prev_ts, now);
        // Mar 01 2023 10:05:00 UTC
        expected = Timestamp::try_from(i64::pow(10, 9) * 1677665100).unwrap();
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_compute_next_ts_daily() {
        // Every day at 8:30
        let cron_spec = CronSpec {
            udf_path: UdfPath::from_str("test").unwrap().canonicalize(),
            udf_args: ConvexArray::try_from(vec![]).unwrap(),
            cron_schedule: CronSchedule::Daily {
                hour_utc: 8,
                minute_utc: 30,
            },
        };

        // Feb 28 2023 08:35:00 UTC
        let now = Timestamp::try_from(i64::pow(10, 9) * 1677573300).unwrap();
        let mut prev_ts = None;
        let mut result = compute_next_ts(&cron_spec, prev_ts, now);
        // Mar 01 2023 8:30:00 UTC
        let mut expected = Timestamp::try_from(i64::pow(10, 9) * 1677659400).unwrap();
        assert_eq!(result.unwrap(), expected);

        prev_ts = Some(expected);
        result = compute_next_ts(&cron_spec, prev_ts, now);
        // Mar 02 2023 8:30:00 UTC
        expected = Timestamp::try_from(i64::pow(10, 9) * 1677745800).unwrap();
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_compute_next_ts_weekly() {
        // Every Tuesday at 12:30
        let cron_spec = CronSpec {
            udf_path: UdfPath::from_str("test").unwrap().canonicalize(),
            udf_args: ConvexArray::try_from(vec![]).unwrap(),
            cron_schedule: CronSchedule::Weekly {
                day_of_week: 2,
                hour_utc: 12,
                minute_utc: 30,
            },
        };

        // Feb 28 2023 08:35:00 UTC
        let now = Timestamp::try_from(i64::pow(10, 9) * 1677573300).unwrap();
        let mut prev_ts = None;
        let mut result = compute_next_ts(&cron_spec, prev_ts, now);
        // Feb 28 2023 12:30:00 UTC
        let mut expected = Timestamp::try_from(i64::pow(10, 9) * 1677587400).unwrap();
        assert_eq!(result.unwrap(), expected);

        prev_ts = Some(expected);
        result = compute_next_ts(&cron_spec, prev_ts, now);
        // Mar 07 2023 12:30:00 UTC
        expected = Timestamp::try_from(i64::pow(10, 9) * 1678192200).unwrap();
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_compute_next_ts_monthly() {
        // Every month on the first day at 12:30
        let cron_spec = CronSpec {
            udf_path: UdfPath::from_str("test").unwrap().canonicalize(),
            udf_args: ConvexArray::try_from(vec![]).unwrap(),
            cron_schedule: CronSchedule::Monthly {
                day: 1,
                hour_utc: 12,
                minute_utc: 30,
            },
        };

        // Feb 28 2023 08:35:00 UTC
        let now = Timestamp::try_from(i64::pow(10, 9) * 1677573300).unwrap();
        let mut prev_ts = None;
        let mut result = compute_next_ts(&cron_spec, prev_ts, now);
        // March 1 2023 12:30:00 UTC
        let mut expected = Timestamp::try_from(i64::pow(10, 9) * 1677673800).unwrap();
        assert_eq!(result.unwrap(), expected);

        prev_ts = Some(expected);
        result = compute_next_ts(&cron_spec, prev_ts, now);
        // April 1 2023 12:30:00 UTC
        // fun fact: this also tests that daylight savings was computed correctly
        expected = Timestamp::try_from(i64::pow(10, 9) * 1680352200).unwrap();
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    // Saffron has pretty comprehensive unit tests so this doesn't need to test all
    // the edge cases
    fn test_compute_next_ts_cron() {
        // Every Monday and Friday at 12:00
        let mut cron_spec = CronSpec {
            udf_path: UdfPath::from_str("test").unwrap().canonicalize(),
            udf_args: ConvexArray::try_from(vec![]).unwrap(),
            cron_schedule: CronSchedule::Cron {
                cron_expr: "0 12 * * 1,5".to_string(),
            },
        };

        // Feb 28 2023 08:35:00 UTC
        let now = Timestamp::try_from(i64::pow(10, 9) * 1677573300).unwrap();
        let mut prev_ts = None;
        let mut result = compute_next_ts(&cron_spec, prev_ts, now);
        // March 3 2023 18:00:00 UTC
        let mut expected = Timestamp::try_from(i64::pow(10, 9) * 1677844800).unwrap();
        assert_eq!(result.unwrap(), expected);

        prev_ts = Some(expected);
        result = compute_next_ts(&cron_spec, prev_ts, now);
        // March 6 2023 18:00:00 UTC
        expected = Timestamp::try_from(i64::pow(10, 9) * 1678104000).unwrap();
        assert_eq!(result.unwrap(), expected);

        // Invalid cron, 7 is not a day of the week
        cron_spec = CronSpec {
            udf_path: UdfPath::from_str("test").unwrap().canonicalize(),
            udf_args: ConvexArray::try_from(vec![]).unwrap(),
            cron_schedule: CronSchedule::Cron {
                cron_expr: "0 12 * * 7".to_string(),
            },
        };
        result = compute_next_ts(&cron_spec, prev_ts, now);
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err())
            .contains("Cron Schedule: Cron parsing from Saffron failed"));
    }
}
