use common::{
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentPath,
    },
    execution_context::ExecutionId,
    types::Timestamp,
    RequestId,
};
#[cfg(any(test, feature = "testing"))]
use proptest::prelude::*;
use serde::{
    Deserialize,
    Serialize,
};
use serde_bytes::ByteBuf;
use serde_json::Value as JsonValue;
use value::{
    codegen_convex_serialization,
    ConvexArray,
    DeveloperDocumentId,
};

#[derive(Clone)]
pub struct ScheduledJob {
    /// This ScheduledJob lives the queue / _scheduled_jobs table of the
    /// component that scheduled it. But it can run jobs in a different
    /// component.
    pub path: CanonicalizedComponentFunctionPath,
    pub udf_args_bytes: ByteBuf,

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

    pub attempts: ScheduledJobAttempts,
}

impl ScheduledJob {
    pub fn udf_args(&self) -> anyhow::Result<ConvexArray> {
        let args_json: JsonValue = serde_json::from_slice(&self.udf_args_bytes)?;
        let args = args_json.try_into()?;
        Ok(args)
    }

    pub fn matches_metadata(&self, metadata: &ScheduledJobMetadata) -> bool {
        self.path == metadata.path
            && self.state == metadata.state
            && self.next_ts == metadata.next_ts
            && self.completed_ts == metadata.completed_ts
            && self.original_scheduled_ts == metadata.original_scheduled_ts
            && self.attempts == metadata.attempts
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
/// Corresponds to a document in the `_scheduled_jobs` table.
pub struct ScheduledJobMetadata {
    /// This ScheduledJob lives the queue / _scheduled_jobs table of the
    /// component that scheduled it. But it can run jobs in a different
    /// component.
    pub path: CanonicalizedComponentFunctionPath,
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "prop_oneof![Just(None),\
                             proptest::arbitrary::any_with::<ConvexArray>((0..4).into()).\
                             prop_map(args_to_bytes).prop_filter_map(\"invalid json\", |b| \
                             b.ok()).prop_map(Some)]")
    )]
    pub udf_args_bytes: Option<ByteBuf>,

    /// ID for the document in the `_scheduled_jobs_args` table in the same
    /// namespace as this scheduled job that has the arguments for the job.
    pub args_id: Option<DeveloperDocumentId>,

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

    pub attempts: ScheduledJobAttempts,
}

fn args_to_bytes(args: ConvexArray) -> anyhow::Result<ByteBuf> {
    let args_bytes = args.json_serialize()?.into_bytes();
    Ok(ByteBuf::from(args_bytes))
}

impl ScheduledJobMetadata {
    pub fn new(
        path: CanonicalizedComponentFunctionPath,
        udf_args: ConvexArray,
        args_id: DeveloperDocumentId,
        state: ScheduledJobState,
        next_ts: Option<Timestamp>,
        completed_ts: Option<Timestamp>,
        original_scheduled_ts: Timestamp,
        attempts: ScheduledJobAttempts,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            path,
            udf_args_bytes: Some(args_to_bytes(udf_args)?),
            args_id: Some(args_id),
            state,
            next_ts,
            completed_ts,
            original_scheduled_ts,
            attempts,
        })
    }
}

pub fn args_from_bytes(args_bytes: ByteBuf) -> anyhow::Result<ConvexArray> {
    let args_json: JsonValue = serde_json::from_slice(&args_bytes)?;
    let args = args_json.try_into()?;
    Ok(args)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedScheduledJob {
    component: Option<String>,
    udf_path: String,
    // Serialize the udf arguments as binary since we restrict what
    // field names can be used in a `Document`'s top-level object.
    udf_args: Option<ByteBuf>,
    args_id: Option<String>,
    state: SerializedScheduledJobState,
    next_ts: Option<i64>,
    completed_ts: Option<i64>,
    original_scheduled_ts: Option<i64>,
    attempts: Option<ScheduledJobAttempts>,
}

impl TryFrom<ScheduledJobMetadata> for SerializedScheduledJob {
    type Error = anyhow::Error;

    fn try_from(job: ScheduledJobMetadata) -> anyhow::Result<Self> {
        Ok(SerializedScheduledJob {
            component: Some(String::from(job.path.component)),
            udf_path: String::from(job.path.udf_path),
            udf_args: job.udf_args_bytes,
            args_id: job.args_id.map(|id| id.to_string()),
            state: job.state.try_into()?,
            next_ts: job.next_ts.map(|ts| ts.into()),
            completed_ts: job.completed_ts.map(|ts| ts.into()),
            original_scheduled_ts: Some(job.original_scheduled_ts.into()),
            attempts: Some(job.attempts),
        })
    }
}

impl TryFrom<SerializedScheduledJob> for ScheduledJobMetadata {
    type Error = anyhow::Error;

    fn try_from(value: SerializedScheduledJob) -> anyhow::Result<Self> {
        let component = value
            .component
            .map(|p| p.parse())
            .transpose()?
            .unwrap_or_else(ComponentPath::root);
        let udf_path = value.udf_path.parse()?;
        let udf_args_bytes = value.udf_args;
        let args_id = value.args_id.map(|id| id.parse()).transpose()?;
        let state = value.state.try_into()?;
        let next_ts = value.next_ts.map(|ts| ts.try_into()).transpose()?;
        let completed_ts = value.completed_ts.map(|ts| ts.try_into()).transpose()?;
        let original_scheduled_ts = match value.original_scheduled_ts {
            Some(ts) => ts.try_into()?,
            // We added original_scheduled_ts later, and thus there are some historical pending jobs
            // that don't have it set. In that case, fallback to next_ts, which is the original
            // schedule time.
            None => match next_ts {
                Some(next_ts) => next_ts,
                None => {
                    anyhow::bail!("Could not use next_ts as a fallback for original_scheduled_ts")
                },
            },
        };

        Ok(ScheduledJobMetadata {
            path: CanonicalizedComponentFunctionPath {
                component,
                udf_path,
            },
            udf_args_bytes,
            args_id,
            state,
            next_ts,
            completed_ts,
            original_scheduled_ts,
            attempts: value.attempts.unwrap_or_default(),
        })
    }
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[serde(rename_all = "camelCase")]
pub struct ScheduledJobAttempts {
    pub system_errors: u32,
    pub occ_errors: u32,
}

impl ScheduledJobAttempts {
    pub fn count_failures(&self) -> u32 {
        self.system_errors + self.occ_errors
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
    ///
    /// TODO: remove `None` case for scheduled jobs that started before we
    /// started recording execution id.
    InProgress {
        request_id: Option<RequestId>,
        execution_id: Option<ExecutionId>,
    },

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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum SerializedScheduledJobState {
    Pending,
    InProgress {
        request_id: Option<RequestId>,
        execution_id: Option<ExecutionId>,
    },
    Success,
    Failed {
        error: String,
    },
    Canceled,
}

impl TryFrom<ScheduledJobState> for SerializedScheduledJobState {
    type Error = anyhow::Error;

    fn try_from(state: ScheduledJobState) -> anyhow::Result<Self> {
        match state {
            ScheduledJobState::Pending => Ok(SerializedScheduledJobState::Pending),
            ScheduledJobState::InProgress {
                request_id,
                execution_id,
            } => Ok(SerializedScheduledJobState::InProgress {
                request_id,
                execution_id,
            }),
            ScheduledJobState::Success => Ok(SerializedScheduledJobState::Success),
            ScheduledJobState::Failed(e) => Ok(SerializedScheduledJobState::Failed { error: e }),
            ScheduledJobState::Canceled => Ok(SerializedScheduledJobState::Canceled),
        }
    }
}

impl TryFrom<SerializedScheduledJobState> for ScheduledJobState {
    type Error = anyhow::Error;

    fn try_from(value: SerializedScheduledJobState) -> anyhow::Result<Self> {
        match value {
            SerializedScheduledJobState::Pending => Ok(ScheduledJobState::Pending),
            SerializedScheduledJobState::InProgress {
                request_id,
                execution_id,
            } => Ok(ScheduledJobState::InProgress {
                request_id,
                execution_id,
            }),
            SerializedScheduledJobState::Success => Ok(ScheduledJobState::Success),
            SerializedScheduledJobState::Failed { error } => Ok(ScheduledJobState::Failed(error)),
            SerializedScheduledJobState::Canceled => Ok(ScheduledJobState::Canceled),
        }
    }
}

codegen_convex_serialization!(ScheduledJobMetadata, SerializedScheduledJob);

mod state {
    use value::codegen_convex_serialization;

    use super::{
        ScheduledJobState,
        SerializedScheduledJobState,
    };

    codegen_convex_serialization!(ScheduledJobState, SerializedScheduledJobState);
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct ScheduledJobArgs {
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(
            strategy = "proptest::arbitrary::any_with::<ConvexArray>((0..4).into()).\
                        prop_map(args_to_bytes).prop_filter_map(\"invalid json\", |b| b.ok())"
        )
    )]
    pub args: ByteBuf,
}

impl TryFrom<ConvexArray> for ScheduledJobArgs {
    type Error = anyhow::Error;

    fn try_from(args: ConvexArray) -> anyhow::Result<Self> {
        Ok(ScheduledJobArgs {
            args: args_to_bytes(args)?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedScheduledJobArgs {
    args: ByteBuf,
}

impl From<ScheduledJobArgs> for SerializedScheduledJobArgs {
    fn from(value: ScheduledJobArgs) -> SerializedScheduledJobArgs {
        SerializedScheduledJobArgs { args: value.args }
    }
}

impl TryFrom<SerializedScheduledJobArgs> for ScheduledJobArgs {
    type Error = anyhow::Error;

    fn try_from(value: SerializedScheduledJobArgs) -> anyhow::Result<Self> {
        Ok(ScheduledJobArgs { args: value.args })
    }
}

codegen_convex_serialization!(ScheduledJobArgs, SerializedScheduledJobArgs);
