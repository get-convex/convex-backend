use std::{
    borrow::Borrow,
    collections::BTreeMap,
    mem,
    ops::Deref,
    str::FromStr,
};

use anyhow::{
    bail,
    Context,
};
use common::{
    log_lines::RawLogLines,
    types::Timestamp,
};
use saffron::Cron;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value as JsonValue;
use sync_types::{
    CanonicalizedUdfPath,
    UdfPath,
};
use value::{
    heap_size::HeapSize,
    json_deserialize,
    json_serialize,
    obj,
    ConvexArray,
    ConvexObject,
    ConvexValue,
};

#[derive(thiserror::Error, Debug, Clone)]
pub enum CronValidationError {
    #[error("Invalid JSON")]
    InvalidJson,
    #[error("Exactly one of (seconds, minutes, hours) should be specified")]
    SecondsMinutesHours,
    #[error("Interval must be an integer greater than 0")]
    InvalidIntervalValue,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct CronJob {
    // Unique identifier of a cron
    pub name: CronIdentifier,

    // Cron-related metadata specified by the user and updated on pushes
    pub cron_spec: CronSpec,

    // Internally tracked metadata to execute the current run of the cron
    pub state: CronJobState,
    pub prev_ts: Option<Timestamp>,
    pub next_ts: Timestamp,
}

impl TryFrom<CronJob> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(job: CronJob) -> anyhow::Result<Self, Self::Error> {
        let prev_ts = match job.prev_ts {
            None => ConvexValue::Null,
            Some(ts) => ConvexValue::Int64(ts.into()),
        };
        obj!(
            "name" => job.name.to_string(),
            "cronSpec" => ConvexValue::Object(job.cron_spec.try_into()?),
            "state" => ConvexValue::Object(job.state.try_into()?),
            "prevTs" => prev_ts,
            "nextTs" => ConvexValue::Int64(job.next_ts.into()),
        )
    }
}

impl TryFrom<ConvexObject> for CronJob {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> anyhow::Result<Self, Self::Error> {
        let mut fields: BTreeMap<_, _> = value.into();

        let name = match fields.remove("name") {
            Some(ConvexValue::String(s)) => CronIdentifier::from_str(s.to_string().as_str())?,
            _ => anyhow::bail!("Missing or invalid `name` field for CronJob: {:?}", fields),
        };

        let cron_spec = match fields.remove("cronSpec") {
            Some(ConvexValue::Object(o)) => o.try_into()?,
            _ => anyhow::bail!(
                "Missing or invalid `cronSpec` field for CronJob: {:?}",
                fields
            ),
        };
        let state = match fields.remove("state") {
            Some(ConvexValue::Object(o)) => o.try_into()?,
            _ => anyhow::bail!("Missing or invalid `state` field for CronJob: {:?}", fields),
        };

        let prev_ts = match fields.remove("prevTs") {
            Some(ConvexValue::Int64(ts)) => Some(ts.try_into()?),
            Some(ConvexValue::Null) => None,
            _ => anyhow::bail!(
                "Missing or invalid `nextTs` field for CronJob: {:?}",
                fields
            ),
        };

        let next_ts = match fields.remove("nextTs") {
            Some(ConvexValue::Int64(ts)) => ts.try_into()?,
            _ => anyhow::bail!(
                "Missing or invalid `nextTs` field for CronJob: {:?}",
                fields
            ),
        };

        Ok(Self {
            name,
            cron_spec,
            state,
            prev_ts,
            next_ts,
        })
    }
}

/// Check that a string can be used as a CronIdentifier.
pub fn check_valid_cron_identifier(s: &str) -> anyhow::Result<()> {
    for c in s.chars() {
        if !c.is_ascii() || c.is_ascii_control() {
            bail!(
                "CronIdentifier {s} has invalid character '{c}': CronIdentifiers can only contain \
                 ASCII characters that are not control characters"
            );
        }
    }
    Ok(())
}

/// Identifiers of CronSpecs, names in CronJob.
#[derive(Hash, Eq, Ord, PartialEq, PartialOrd, Clone, Debug, derive_more::Display)]
pub struct CronIdentifier(String);

impl HeapSize for CronIdentifier {
    fn heap_size(&self) -> usize {
        self.0.heap_size()
    }
}

impl FromStr for CronIdentifier {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        check_valid_cron_identifier(s)?;
        Ok(Self(s.to_owned()))
    }
}

impl Deref for CronIdentifier {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Borrow<str> for CronIdentifier {
    fn borrow(&self) -> &str {
        &self.0
    }
}

pub const CRON_IDENTIFIER_REGEX: &str = "[-_ 'a-zA-Z]+";

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for CronIdentifier {
    type Parameters = ();

    type Strategy = impl proptest::strategy::Strategy<Value = CronIdentifier>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::strategy::Strategy;
        CRON_IDENTIFIER_REGEX.prop_filter_map("Generated invalid CronIdentifier", |s| {
            CronIdentifier::from_str(&s).ok()
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct CronSpec {
    pub udf_path: CanonicalizedUdfPath,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "proptest::arbitrary::any_with::<ConvexArray>((0..4).into())")
    )]
    pub udf_args: ConvexArray,
    pub cron_schedule: CronSchedule,
}

impl HeapSize for CronSpec {
    fn heap_size(&self) -> usize {
        self.udf_args.heap_size() + self.cron_schedule.heap_size() + self.udf_path.heap_size()
    }
}

impl TryFrom<CronSpec> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(spec: CronSpec) -> anyhow::Result<Self, Self::Error> {
        // Serialize the udf arguments as binary since we restrict what
        // field names can be used in a `Document`'s top-level object.
        let udf_args_json = JsonValue::from(spec.udf_args);
        let udf_args_bytes = serde_json::to_vec(&udf_args_json)?;
        obj!(
            "udfPath" => String::from(spec.udf_path),
            "udfArgs" => udf_args_bytes,
            "cronSchedule" => ConvexValue::Object(spec.cron_schedule.try_into()?),
        )
    }
}

impl TryFrom<ConvexObject> for CronSpec {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> anyhow::Result<Self, Self::Error> {
        let mut fields: BTreeMap<_, _> = value.into();

        let udf_path = match fields.remove("udfPath") {
            Some(ConvexValue::String(s)) => s,
            _ => anyhow::bail!(
                "Missing or invalid `udfPath` field for CronJob: {:?}",
                fields
            ),
        };
        let udf_path: CanonicalizedUdfPath = udf_path
            .parse()
            .context(format!("Failed to deserialize udf_path {}", udf_path))?;
        let udf_args = match fields.remove("udfArgs") {
            Some(ConvexValue::Bytes(b)) => {
                let udf_args_json: JsonValue = serde_json::from_slice(&b)?;
                udf_args_json.try_into()?
            },
            None => ConvexArray::try_from(vec![])?,
            _ => anyhow::bail!(
                "Missing or invalid `udfArgs` field for CronJob: {:?}",
                fields
            ),
        };

        let cron_schedule = match fields.remove("cronSchedule") {
            Some(ConvexValue::Object(o)) => o.try_into()?,
            _ => anyhow::bail!(
                "Missing or invalid `cronSchedule` field for CronJob: {:?}",
                fields
            ),
        };

        Ok(Self {
            udf_path,
            udf_args,
            cron_schedule,
        })
    }
}

impl TryFrom<JsonValue> for CronSpec {
    type Error = anyhow::Error;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        enum DayOfWeek {
            Sunday,
            Monday,
            Tuesday,
            Wednesday,
            Thursday,
            Friday,
            Saturday,
        }

        #[derive(Deserialize)]
        #[serde(tag = "type")]
        enum ScheduleJson {
            #[serde(rename = "interval")]
            Interval {
                seconds: Option<i64>,
                minutes: Option<i64>,
                hours: Option<i64>,
            },
            #[serde(rename = "hourly")]
            Hourly {
                #[serde(rename = "minuteUTC")]
                minute_utc: i64,
            },
            #[serde(rename = "daily")]
            Daily {
                #[serde(rename = "minuteUTC")]
                minute_utc: i64,
                #[serde(rename = "hourUTC")]
                hour_utc: i64,
            },
            #[serde(rename_all = "camelCase")]
            #[serde(rename = "weekly")]
            Weekly {
                #[serde(rename = "minuteUTC")]
                minute_utc: i64,
                #[serde(rename = "hourUTC")]
                hour_utc: i64,
                day_of_week: DayOfWeek,
            },
            #[serde(rename_all = "camelCase")]
            #[serde(rename = "monthly")]
            Monthly {
                #[serde(rename = "minuteUTC")]
                minute_utc: i64,
                #[serde(rename = "hourUTC")]
                hour_utc: i64,
                day: i64,
            },
            #[serde(rename_all = "camelCase")]
            #[serde(rename = "cron")]
            Cron { cron: String },
        }

        // The JavaScript object produced by crons.export() uses different names:
        // name -> udf_path, schedule -> cron_schedule, args -> udf_args
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct CronSpecJson {
            name: String,
            args: JsonValue,
            schedule: ScheduleJson,
        }
        let j: CronSpecJson = serde_json::from_value(value.clone())
            .with_context(|| CronValidationError::InvalidJson)?;

        let schedule = match j.schedule {
            ScheduleJson::Interval {
                seconds,
                minutes,
                hours,
            } => {
                let num_time_fields =
                    seconds.is_some() as i64 + minutes.is_some() as i64 + hours.is_some() as i64;
                if num_time_fields != 1 {
                    anyhow::bail!(CronValidationError::SecondsMinutesHours);
                }
                if let Some(seconds) = seconds {
                    if seconds <= 0 {
                        anyhow::bail!(CronValidationError::InvalidIntervalValue)
                    }
                }
                if let Some(minutes) = minutes {
                    if minutes <= 0 {
                        anyhow::bail!(CronValidationError::InvalidIntervalValue)
                    }
                }
                if let Some(hours) = hours {
                    if hours <= 0 {
                        anyhow::bail!(CronValidationError::InvalidIntervalValue)
                    }
                }
                let seconds =
                    seconds.unwrap_or(0) + minutes.unwrap_or(0) * 60 + hours.unwrap_or(0) * 3600;

                CronSchedule::Interval { seconds }
            },
            ScheduleJson::Hourly { minute_utc } => {
                if !(0..=59).contains(&minute_utc) {
                    anyhow::bail!(
                        "minuteUTC must be 0-59 in {}",
                        serde_json::to_string_pretty(&value).unwrap()
                    );
                }
                CronSchedule::Hourly { minute_utc }
            },
            ScheduleJson::Daily {
                minute_utc,
                hour_utc,
            } => {
                if !(0..=59).contains(&minute_utc) {
                    anyhow::bail!(
                        "minuteUTC must be 0-59 in {}",
                        serde_json::to_string_pretty(&value).unwrap()
                    );
                }
                if !(0..=23).contains(&hour_utc) {
                    anyhow::bail!(
                        "hourUTC must be 0-23 in {}",
                        serde_json::to_string_pretty(&value).unwrap()
                    );
                }
                CronSchedule::Daily {
                    minute_utc,
                    hour_utc,
                }
            },
            ScheduleJson::Weekly {
                minute_utc,
                hour_utc,
                day_of_week,
            } => {
                if !(0..=59).contains(&minute_utc) {
                    anyhow::bail!(
                        "minuteUTC must be 0-59 in {}",
                        serde_json::to_string_pretty(&value).unwrap()
                    );
                }
                if !(0..=23).contains(&hour_utc) {
                    anyhow::bail!(
                        "hourUTC must be 0-23 in {}",
                        serde_json::to_string_pretty(&value).unwrap()
                    );
                }
                CronSchedule::Weekly {
                    minute_utc,
                    hour_utc,
                    day_of_week: match day_of_week {
                        DayOfWeek::Sunday => 0,
                        DayOfWeek::Monday => 1,
                        DayOfWeek::Tuesday => 2,
                        DayOfWeek::Wednesday => 3,
                        DayOfWeek::Thursday => 4,
                        DayOfWeek::Friday => 5,
                        DayOfWeek::Saturday => 6,
                    },
                }
            },
            ScheduleJson::Monthly {
                minute_utc,
                hour_utc,
                day,
            } => {
                if !(0..=59).contains(&minute_utc) {
                    anyhow::bail!(
                        "minuteUTC must be 0-59 in {}",
                        serde_json::to_string_pretty(&value).unwrap()
                    );
                }
                if !(0..=23).contains(&hour_utc) {
                    anyhow::bail!(
                        "hourUTC must be 0-23 in {}",
                        serde_json::to_string_pretty(&value).unwrap()
                    );
                }
                if !(1..=31).contains(&day) {
                    anyhow::bail!(
                        "day of month must be 1-31 in {}",
                        serde_json::to_string_pretty(&value).unwrap()
                    );
                }
                CronSchedule::Monthly {
                    day,
                    hour_utc,
                    minute_utc,
                }
            },
            ScheduleJson::Cron { cron } => {
                cron.parse::<saffron::Cron>()?;
                CronSchedule::Cron { cron_expr: cron }
            },
        };

        let udf_path: UdfPath = j.name.parse()?;
        let udf_path_canonicalized = udf_path.canonicalize();
        Ok(Self {
            udf_path: udf_path_canonicalized,
            udf_args: ConvexArray::try_from(j.args)?,
            cron_schedule: schedule,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum CronJobState {
    // Yet to be attempted.
    Pending,
    // Started but not completed yet. Used to make actions execute at most once.
    InProgress,
}

impl TryFrom<CronJobState> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(state: CronJobState) -> anyhow::Result<Self, Self::Error> {
        match state {
            CronJobState::Pending => obj!("type" => "pending"),
            CronJobState::InProgress => obj!("type" => "inProgress"),
        }
    }
}

impl TryFrom<ConvexObject> for CronJobState {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> anyhow::Result<Self, Self::Error> {
        let mut fields: BTreeMap<_, _> = value.into();
        let state_t = match fields.remove("type") {
            Some(ConvexValue::String(s)) => s,
            _ => anyhow::bail!(
                "Missing or invalid `type` field for CronJobState: {:?}",
                fields
            ),
        };

        let state = match state_t.as_ref() {
            "pending" => CronJobState::Pending,
            "inProgress" => CronJobState::InProgress,
            _ => anyhow::bail!("Invalid CronJobState `type`: {}", state_t),
        };
        Ok(state)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum CronSchedule {
    Interval {
        seconds: i64,
    },
    Hourly {
        minute_utc: i64,
    },
    Daily {
        hour_utc: i64,
        minute_utc: i64,
    },
    Weekly {
        day_of_week: i64,
        hour_utc: i64,
        minute_utc: i64,
    },
    Monthly {
        day: i64,
        hour_utc: i64,
        minute_utc: i64,
    },
    Cron {
        cron_expr: String,
    },
}

impl HeapSize for CronSchedule {
    fn heap_size(&self) -> usize {
        match self {
            CronSchedule::Interval { .. } => mem::size_of::<i64>(),
            CronSchedule::Hourly { .. } => mem::size_of::<i64>(),
            CronSchedule::Daily { .. } => 2 * mem::size_of::<i64>(),
            CronSchedule::Monthly { .. } | CronSchedule::Weekly { .. } => 3 * mem::size_of::<i64>(),
            CronSchedule::Cron { cron_expr } => cron_expr.heap_size(),
        }
    }
}

impl TryFrom<CronSchedule> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(cron_schedule: CronSchedule) -> anyhow::Result<Self, Self::Error> {
        match cron_schedule {
            CronSchedule::Interval { seconds } => obj!(
                "type" => "interval",
                "seconds" => seconds,
            ),
            CronSchedule::Hourly { minute_utc } => obj!(
                "type" => "hourly",
                "minuteUTC" => minute_utc,
            ),
            CronSchedule::Daily {
                hour_utc,
                minute_utc,
            } => obj!(
                "type" => "daily",
                "hourUTC" => hour_utc,
                "minuteUTC" => minute_utc,
            ),
            CronSchedule::Weekly {
                day_of_week,
                hour_utc,
                minute_utc,
            } => obj!(
                "type" => "weekly",
                "dayOfWeek" => day_of_week,
                "hourUTC" => hour_utc,
                "minuteUTC" => minute_utc,
            ),
            CronSchedule::Monthly {
                day,
                hour_utc,
                minute_utc,
            } => obj!(
                "type" => "monthly",
                "day" => day,
                "hourUTC" => hour_utc,
                "minuteUTC" => minute_utc,
            ),
            CronSchedule::Cron { cron_expr } => obj!(
                "type" => "cron",
                "cronExpr" => cron_expr,
            ),
        }
    }
}

impl TryFrom<ConvexObject> for CronSchedule {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> anyhow::Result<Self, Self::Error> {
        let mut fields: BTreeMap<_, _> = value.into();
        let type_t = match fields.remove("type") {
            Some(ConvexValue::String(s)) => s,
            _ => anyhow::bail!(
                "Missing or invalid `type` field for CronSchedule: {:?}",
                fields
            ),
        };

        let schedule = match type_t.as_ref() {
            "interval" => {
                let seconds = match fields.remove("seconds") {
                    Some(ConvexValue::Int64(i)) => i,
                    _ => anyhow::bail!(
                        "Missing or invalid `seconds` field for CronSchedule Interval: {:?}",
                        fields
                    ),
                };
                CronSchedule::Interval { seconds }
            },
            "hourly" => {
                let minute_utc = match fields.remove("minuteUTC") {
                    Some(ConvexValue::Int64(i)) => i,
                    _ => anyhow::bail!(
                        "Missing or invalid `minute_utc` field for CronSchedule Hourly: {:?}",
                        fields
                    ),
                };
                CronSchedule::Hourly { minute_utc }
            },
            "daily" => {
                let hour_utc = match fields.remove("hourUTC") {
                    Some(ConvexValue::Int64(i)) => i,
                    _ => anyhow::bail!(
                        "Missing or invalid `hour_utc` field for CronSchedule Daily: {:?}",
                        fields
                    ),
                };
                let minute_utc = match fields.remove("minuteUTC") {
                    Some(ConvexValue::Int64(i)) => i,
                    _ => anyhow::bail!(
                        "Missing or invalid `minute_utc` field for CronSchedule Daily: {:?}",
                        fields
                    ),
                };
                CronSchedule::Daily {
                    hour_utc,
                    minute_utc,
                }
            },
            "weekly" => {
                let day_of_week = match fields.remove("dayOfWeek") {
                    Some(ConvexValue::Int64(i)) => i,
                    _ => anyhow::bail!(
                        "Missing or invalid `day_of_week` field for CronSchedule Weekly: {:?}",
                        fields
                    ),
                };
                let hour_utc = match fields.remove("hourUTC") {
                    Some(ConvexValue::Int64(i)) => i,
                    _ => anyhow::bail!(
                        "Missing or invalid `hour_utc` field for CronSchedule Weekly: {:?}",
                        fields
                    ),
                };
                let minute_utc = match fields.remove("minuteUTC") {
                    Some(ConvexValue::Int64(i)) => i,
                    _ => anyhow::bail!(
                        "Missing or invalid `minute_utc` field for CronSchedule Weekly: {:?}",
                        fields
                    ),
                };
                CronSchedule::Weekly {
                    day_of_week,
                    hour_utc,
                    minute_utc,
                }
            },
            "monthly" => {
                let day = match fields.remove("day") {
                    Some(ConvexValue::Int64(i)) => i,
                    _ => anyhow::bail!(
                        "Missing or invalid `day` field for CronSchedule Monthly: {:?}",
                        fields
                    ),
                };
                let hour_utc = match fields.remove("hourUTC") {
                    Some(ConvexValue::Int64(i)) => i,
                    _ => anyhow::bail!(
                        "Missing or invalid `hour_utc` field for CronSchedule Monthly: {:?}",
                        fields
                    ),
                };
                let minute_utc = match fields.remove("minuteUTC") {
                    Some(ConvexValue::Int64(i)) => i,
                    _ => anyhow::bail!(
                        "Missing or invalid `minute_utc` field for CronSchedule Weekly: {:?}",
                        fields
                    ),
                };
                CronSchedule::Monthly {
                    day,
                    hour_utc,
                    minute_utc,
                }
            },
            "cron" => {
                let cron_expr: String = match fields.remove("cronExpr") {
                    Some(s) => s.try_into()?,
                    _ => anyhow::bail!(
                        "Missing or invalid `cron_expr` field for CronSchedule Cron: {:?}",
                        fields
                    ),
                };
                CronSchedule::Cron { cron_expr }
            },
            _ => anyhow::bail!("Invalid CronSchedule `type`: {}", type_t),
        };
        Ok(schedule)
    }
}

impl CronSchedule {
    pub fn validate_format(&self) -> anyhow::Result<()> {
        let _: Cron = match self.clone() {
            CronSchedule::Interval { seconds } => {
                if seconds <= 0 {
                    bail!("CronSchedule intervals must have a positive duration.");
                }
                return Ok(());
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
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct CronJobLog {
    pub name: CronIdentifier,
    pub ts: Timestamp,
    pub udf_path: CanonicalizedUdfPath,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "proptest::arbitrary::any_with::<ConvexArray>((0..4).into())")
    )]
    pub udf_args: ConvexArray,
    pub status: CronJobStatus,
    pub log_lines: CronJobLogLines,
    pub execution_time: f64,
}

impl TryFrom<CronJobLog> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(log: CronJobLog) -> anyhow::Result<Self, Self::Error> {
        // Serialize the udf arguments as binary since we restrict what
        // field names can be used in a `Document`'s top-level object.
        let udf_args_json = JsonValue::from(log.udf_args);
        let udf_args_bytes = serde_json::to_vec(&udf_args_json)?;

        obj!(
            "name" => log.name.to_string(),
            "ts" => ConvexValue::Int64(log.ts.into()),
            "udfPath" => String::from(log.udf_path),
            "udfArgs" => udf_args_bytes,
            "status" => ConvexValue::Object(log.status.try_into()?),
            "logLines" => ConvexValue::Object(log.log_lines.try_into()?),
            "executionTime" => log.execution_time,
        )
    }
}

impl TryFrom<ConvexObject> for CronJobLog {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> anyhow::Result<Self, Self::Error> {
        let mut fields: BTreeMap<_, _> = value.into();

        let name = match fields.remove("name") {
            Some(ConvexValue::String(s)) => CronIdentifier::from_str(s.to_string().as_str())?,
            _ => anyhow::bail!(
                "Missing or invalid `name` field for CronJobLog: {:?}",
                fields
            ),
        };
        let ts = match fields.remove("ts") {
            Some(ConvexValue::Int64(ts)) => ts.try_into()?,
            _ => anyhow::bail!("Missing or invalid `ts` field for CronJobLog: {:?}", fields),
        };
        let udf_path = match fields.remove("udfPath") {
            Some(ConvexValue::String(s)) => s,
            _ => anyhow::bail!(
                "Missing or invalid `udfPath` field for CronJobLog: {:?}",
                fields
            ),
        };
        let udf_path: CanonicalizedUdfPath = udf_path
            .parse()
            .context(format!("Failed to deserialize udf_path {}", udf_path))?;
        let udf_args = match fields.remove("udfArgs") {
            Some(ConvexValue::Bytes(b)) => {
                let udf_args_json: JsonValue = serde_json::from_slice(&b)?;
                udf_args_json.try_into()?
            },
            _ => anyhow::bail!(
                "Missing or invalid `udfArgs` field for CronJobLog: {:?}",
                fields
            ),
        };
        let status = match fields.remove("status") {
            Some(ConvexValue::Object(o)) => o.try_into()?,
            _ => anyhow::bail!(
                "Missing or invalid `status` field for CronJobLog: {:?}",
                fields
            ),
        };
        let log_lines = match fields.remove("logLines") {
            Some(ConvexValue::Object(o)) => o.try_into()?,
            _ => anyhow::bail!(
                "Missing or invalid `logLines` field for CronJobLog: {:?}",
                fields
            ),
        };
        let execution_time = match fields.remove("executionTime") {
            Some(ConvexValue::Float64(ts)) => ts,
            _ => anyhow::bail!(
                "Missing or invalid `executionTime` field for CronJobLog: {:?}",
                fields
            ),
        };

        Ok(Self {
            name,
            ts,
            udf_path,
            udf_args,
            status,
            log_lines,
            execution_time,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum CronJobStatus {
    Success(CronJobResult),
    Err(String),
    Canceled { num_canceled: i64 },
}

impl TryFrom<CronJobStatus> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(status: CronJobStatus) -> anyhow::Result<Self, Self::Error> {
        match status {
            CronJobStatus::Success(r) => {
                obj!(
                    "type" => "success",
                    "result" => ConvexValue::Object(r.try_into()?),
                )
            },
            CronJobStatus::Err(e) => {
                obj!("type" => "err", "error" => e)
            },
            CronJobStatus::Canceled { num_canceled } => {
                obj!("type" => "canceled", "num_canceled" => num_canceled)
            },
        }
    }
}

impl TryFrom<ConvexObject> for CronJobStatus {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> anyhow::Result<Self, Self::Error> {
        let mut fields: BTreeMap<_, _> = value.into();
        let status_t = match fields.remove("type") {
            Some(ConvexValue::String(s)) => s,
            _ => anyhow::bail!(
                "Missing or invalid `type` field for CronJobStatus: {:?}",
                fields
            ),
        };

        return match status_t.as_ref() {
            "success" => {
                let result = match fields.remove("result") {
                    Some(ConvexValue::Object(o)) => o.try_into()?,
                    _ => anyhow::bail!(
                        "Missing or invalid `value` field for CronJobStatus: {:?}",
                        fields
                    ),
                };
                Ok(CronJobStatus::Success(result))
            },
            "err" => {
                let err = match fields.remove("error") {
                    Some(ConvexValue::String(s)) => s,
                    _ => anyhow::bail!(
                        "Missing or invalid `error` field for CronJobStatus: {:?}",
                        fields
                    ),
                };
                Ok(CronJobStatus::Err(err.into()))
            },
            "canceled" => {
                let num_canceled = match fields.remove("num_canceled") {
                    Some(ConvexValue::Int64(i)) => i,
                    _ => anyhow::bail!(
                        "Missing or invalid `num_canceled` field for CronJobStatus: {:?}",
                        fields
                    ),
                };
                Ok(CronJobStatus::Canceled { num_canceled })
            },
            _ => anyhow::bail!("Invalid CronJobStatus `type`: {}", status_t),
        };
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum CronJobResult {
    Default(ConvexValue),
    Truncated(String),
}

impl TryFrom<CronJobResult> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(result: CronJobResult) -> anyhow::Result<Self, Self::Error> {
        match result {
            CronJobResult::Default(v) => {
                obj!(
                    "type" => "default",
                    "value" => json_serialize(v)?,
                )
            },
            CronJobResult::Truncated(s) => {
                obj!("type" => "truncated", "truncated_log" => s)
            },
        }
    }
}

impl TryFrom<ConvexObject> for CronJobResult {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> anyhow::Result<Self, Self::Error> {
        let mut fields: BTreeMap<_, _> = value.into();
        let result_t = match fields.remove("type") {
            Some(ConvexValue::String(s)) => s,
            _ => anyhow::bail!(
                "Missing or invalid `type` field for CronJobResult: {:?}",
                fields
            ),
        };

        return match result_t.as_ref() {
            "default" => {
                let value = match fields.remove("value") {
                    Some(ConvexValue::String(s)) => json_deserialize(&s)?,
                    _ => anyhow::bail!(
                        "Missing or invalid `value` field for CronJobResult: {:?}",
                        fields
                    ),
                };
                Ok(CronJobResult::Default(value))
            },
            "truncated" => {
                let truncated_log = match fields.remove("truncated_log") {
                    Some(ConvexValue::String(s)) => s,
                    _ => anyhow::bail!(
                        "Missing or invalid `truncated_log` field for CronJobResult: {:?}",
                        fields
                    ),
                };
                Ok(CronJobResult::Truncated(truncated_log.into()))
            },
            _ => anyhow::bail!("Invalid CronJobResult `type`: {}", result_t),
        };
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct CronJobLogLines {
    pub log_lines: RawLogLines,
    pub is_truncated: bool,
}

impl TryFrom<CronJobLogLines> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(cron_log: CronJobLogLines) -> anyhow::Result<Self, Self::Error> {
        let log_lines: anyhow::Result<Vec<ConvexValue>> = cron_log
            .log_lines
            .into_iter()
            .map(ConvexValue::try_from)
            .collect();
        obj!(
            "logLines" => ConvexValue::Array(log_lines?.try_into()?),
            "isTruncated" => ConvexValue::Boolean(cron_log.is_truncated),
        )
    }
}

impl TryFrom<ConvexObject> for CronJobLogLines {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> anyhow::Result<Self, Self::Error> {
        let mut fields: BTreeMap<_, _> = value.into();
        let log_lines: RawLogLines = match fields.remove("logLines") {
            Some(ConvexValue::Array(a)) => a
                .into_iter()
                .map(String::try_from)
                .collect::<anyhow::Result<RawLogLines>>()?,
            _ => anyhow::bail!(
                "Missing or invalid `logLines` field for CronJobLogLines: {:?}",
                fields
            ),
        };
        let is_truncated = match fields.remove("isTruncated") {
            Some(ConvexValue::Boolean(b)) => b,
            _ => anyhow::bail!(
                "Missing or invalid `isTruncated` field for CronJobLogLines: {:?}",
                fields
            ),
        };
        Ok(CronJobLogLines {
            log_lines,
            is_truncated,
        })
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use sync_types::testing::assert_roundtrips;
    use value::ConvexObject;

    use crate::cron_jobs::types::{
        CronJob,
        CronJobLog,
        CronJobLogLines,
        CronJobResult,
        CronJobStatus,
        CronSchedule,
        CronSpec,
    };

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_cron_job_roundtrips(v in any::<CronJob>()) {
            assert_roundtrips::<CronJob, ConvexObject>(v);
        }
    }

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_cron_spec_roundtrips(v in any::<CronSpec>()) {
            assert_roundtrips::<CronSpec, ConvexObject>(v);
        }
    }

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_cron_schedule_roundtrips(v in any::<CronSchedule>()) {
            assert_roundtrips::<CronSchedule, ConvexObject>(v);
        }
    }

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_cron_job_log_roundtrips(v in any::<CronJobLog>()) {
            assert_roundtrips::<CronJobLog, ConvexObject>(v);
        }
    }

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_cron_job_status_roundtrips(v in any::<CronJobStatus>()) {
            assert_roundtrips::<CronJobStatus, ConvexObject>(v);
        }
    }

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_cron_job_result_roundtrips(v in any::<CronJobResult>()) {
            assert_roundtrips::<CronJobResult, ConvexObject>(v);
        }
    }

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_cron_job_log_lines_roundtrips(v in any::<CronJobLogLines>()) {
            assert_roundtrips::<CronJobLogLines, ConvexObject>(v);
        }
    }
}
