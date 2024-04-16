use std::collections::BTreeMap;

use anyhow::Context;
use common::types::Timestamp;
#[cfg(any(test, feature = "testing"))]
use proptest::prelude::*;
use serde_json::Value as JsonValue;
use sync_types::CanonicalizedUdfPath;
use value::{
    obj,
    ConvexArray,
    ConvexObject,
    ConvexValue,
    FieldName,
};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct ScheduledJob {
    pub udf_path: CanonicalizedUdfPath,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "proptest::arbitrary::any_with::<ConvexArray>((0..4).into())")
    )]
    pub udf_args: ConvexArray,

    pub state: ScheduledJobState,

    // next_ts is the timestamp when the job was scheduled, and should only be set on pending and
    // in-progress states. completed_ts is the timestamp when the job was completed, and should
    // only be set on success, failed, and canceled states. This allows us to use an index to find
    // jobs that still need to be processed and jobs that can be garbage collected without doing
    // multiple queries on different states and merging the results. original_scheduled_ts is the
    // timestamp when the job was scheduled, but does not get mutated as the job transitions
    // between states.
    pub next_ts: Option<Timestamp>,
    pub completed_ts: Option<Timestamp>,
    pub original_scheduled_ts: Timestamp,
}

impl TryFrom<ScheduledJob> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(job: ScheduledJob) -> anyhow::Result<Self> {
        // Serialize the udf arguments as binary since we restrict what
        // field names can be used in a `Document`'s top-level object.
        let udf_args_json = JsonValue::from(job.udf_args);
        let udf_args_bytes = serde_json::to_vec(&udf_args_json)?;
        let mut obj: BTreeMap<FieldName, ConvexValue> = BTreeMap::new();
        obj.insert(
            "udfPath".parse()?,
            ConvexValue::try_from(String::from(job.udf_path))?,
        );
        obj.insert("udfArgs".parse()?, ConvexValue::try_from(udf_args_bytes)?);
        obj.insert("state".parse()?, ConvexValue::Object(job.state.try_into()?));
        if let Some(next_ts) = job.next_ts {
            obj.insert("nextTs".parse()?, ConvexValue::Int64(next_ts.into()));
        }
        if let Some(completed_ts) = job.completed_ts {
            obj.insert(
                "completedTs".parse()?,
                ConvexValue::Int64(completed_ts.into()),
            );
        }
        obj.insert(
            "originalScheduledTs".parse()?,
            ConvexValue::Int64(job.original_scheduled_ts.into()),
        );

        ConvexObject::try_from(obj)
    }
}

impl TryFrom<ConvexObject> for ScheduledJob {
    type Error = anyhow::Error;

    fn try_from(object: ConvexObject) -> anyhow::Result<Self> {
        let mut fields: BTreeMap<_, _> = object.into();

        let udf_path = match fields.remove("udfPath") {
            Some(ConvexValue::String(s)) => s,
            _ => anyhow::bail!(
                "Missing or invalid `udfPath` field for ScheduledJob: {:?}",
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
                "Missing or invalid `udfArgs` field for ScheduledJob: {:?}",
                fields
            ),
        };
        let state = match fields.remove("state") {
            Some(ConvexValue::Object(o)) => o.try_into()?,
            _ => anyhow::bail!(
                "Missing or invalid `state` field for ScheduledJob: {:?}",
                fields
            ),
        };
        let next_ts = match fields.remove("nextTs") {
            Some(ConvexValue::Int64(ts)) => Some(ts.try_into()?),
            _ => None,
        };
        let completed_ts = match fields.remove("completedTs") {
            Some(ConvexValue::Int64(ts)) => Some(ts.try_into()?),
            _ => None,
        };

        let original_scheduled_ts = match fields.remove("originalScheduledTs") {
            Some(ConvexValue::Int64(ts)) => ts.try_into()?,
            // We added original_scheduled_ts later, and thus there are some historical pending jobs
            // that don't have it set. In that case, fallback to next_ts, which is the original
            // schedule time.
            None => match next_ts {
                Some(next_ts) => next_ts,
                None => {
                    anyhow::bail!("Could not use next_ts as a fallback for original_scheduled_ts")
                },
            },
            _ => anyhow::bail!(
                "Missing or invalid `original_scheduled_ts` field for ScheduledJob: {:?}",
                fields
            ),
        };

        Ok(ScheduledJob {
            udf_path,
            udf_args,
            state,
            next_ts,
            completed_ts,
            original_scheduled_ts,
        })
    }
}

/// The state machine for scheduled jobs. Note that only actions go through the
/// InProgress state. Mutations jump straight from Pending to one of the
/// completion states.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum ScheduledJobState {
    /// Job has not started yet.
    Pending,
    /// Job has started running but is not completed yet. This state only
    /// applies to actions, and is used to make actions execute at most once.
    InProgress,

    /// Completion states
    /// Job finished running successully with no errors.
    Success,
    /// Job hit an error while running, which can either be a deterministic user
    /// JS error or an internal error such as a transient error when running
    /// actions or trying to run a function that is not a mutation or action.
    Failed(String),
    /// Job was canceled via the dashboard, ctx.scheduler.cancel, or recursively
    /// by a parent scheduled job that was canceled while in progress.
    Canceled,
}

impl TryFrom<ScheduledJobState> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(state: ScheduledJobState) -> anyhow::Result<Self, Self::Error> {
        match state {
            ScheduledJobState::Pending => obj!("type" => "pending"),
            ScheduledJobState::InProgress => obj!("type" => "inProgress"),
            ScheduledJobState::Success => obj!("type" => "success"),
            ScheduledJobState::Failed(e) => obj!(
                "type" => "failed",
                "error" => e,
            ),
            ScheduledJobState::Canceled => obj!("type" => "canceled"),
        }
    }
}

impl TryFrom<ConvexObject> for ScheduledJobState {
    type Error = anyhow::Error;

    fn try_from(value: ConvexObject) -> anyhow::Result<Self, Self::Error> {
        let mut fields: BTreeMap<_, _> = value.into();
        let state_t = match fields.remove("type") {
            Some(ConvexValue::String(s)) => s,
            _ => anyhow::bail!(
                "Missing or invalid `type` field for ScheduledJobState: {:?}",
                fields
            ),
        };

        match state_t.as_ref() {
            "pending" => Ok(ScheduledJobState::Pending),
            "inProgress" => Ok(ScheduledJobState::InProgress),
            "success" => Ok(ScheduledJobState::Success),
            "failed" => {
                let error = match fields.remove("error") {
                    Some(ConvexValue::String(s)) => s,
                    _ => anyhow::bail!(
                        "Missing or invalid `error` field for ScheduledJobState: {:?}",
                        fields
                    ),
                };
                Ok(ScheduledJobState::Failed(error.to_string()))
            },
            "canceled" => Ok(ScheduledJobState::Canceled),
            _ => anyhow::bail!("Invalid `type` field for ScheduledJobState: {:?}", state_t),
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use value::{
        testing::assert_roundtrips,
        ConvexObject,
    };

    use super::{
        ScheduledJob,
        ScheduledJobState,
    };

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_scheduled_job_roundtrips(v in any::<ScheduledJob>()) {
            assert_roundtrips::<ScheduledJob, ConvexObject>(v);
        }
    }

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn test_scheduled_job_state_roundtrips(v in any::<ScheduledJobState>()) {
            assert_roundtrips::<ScheduledJobState, ConvexObject>(v);
        }
    }
}
