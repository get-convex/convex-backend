use common::identity::InertIdentity;
use pb::funrun::{
    function_outcome::Outcome as OutcomeProto,
    FunctionOutcome as FunctionOutcomeProto,
};

use crate::{
    environment::{
        action::outcome::{
            ActionOutcome,
            HttpActionOutcome,
        },
        udf::outcome::UdfOutcome,
    },
    ValidatedUdfPathAndArgs,
};

/// A `UdfOutcome` represents a successful execution of a developer's function
/// by our V8 layer. It slightly differs from `UdfExecution`, which is what we
/// store in memory for logs.
#[derive(Debug, Clone)]
pub enum FunctionOutcome {
    Query(UdfOutcome),
    Mutation(UdfOutcome),
    Action(ActionOutcome),
    HttpAction(HttpActionOutcome),
}

// TODO: Make the proto self-contained using From<FunctionOutcomeProto>
// instead of requiring path_and_args and identity as additional arguments.
impl FunctionOutcome {
    pub fn from_proto(
        FunctionOutcomeProto { outcome }: FunctionOutcomeProto,
        path_and_args: ValidatedUdfPathAndArgs,
        identity: InertIdentity,
    ) -> anyhow::Result<Self> {
        let outcome = outcome.ok_or_else(|| anyhow::anyhow!("Missing outcome"))?;
        match outcome {
            OutcomeProto::Query(outcome) => Ok(FunctionOutcome::Query(UdfOutcome::from_proto(
                outcome,
                path_and_args,
                identity,
            )?)),
            OutcomeProto::Mutation(outcome) => Ok(FunctionOutcome::Mutation(
                UdfOutcome::from_proto(outcome, path_and_args, identity)?,
            )),
            OutcomeProto::Action(outcome) => Ok(FunctionOutcome::Action(
                ActionOutcome::from_proto(outcome, path_and_args, identity)?,
            )),
        }
    }
}

impl TryFrom<FunctionOutcome> for FunctionOutcomeProto {
    type Error = anyhow::Error;

    fn try_from(value: FunctionOutcome) -> anyhow::Result<Self> {
        let outcome = match value {
            FunctionOutcome::Query(outcome) => OutcomeProto::Query(outcome.try_into()?),
            FunctionOutcome::Mutation(outcome) => OutcomeProto::Mutation(outcome.try_into()?),
            FunctionOutcome::Action(outcome) => OutcomeProto::Action(outcome.try_into()?),
            FunctionOutcome::HttpAction(_) => {
                anyhow::bail!("Funrun does not support http actions")
            },
        };
        Ok(FunctionOutcomeProto {
            outcome: Some(outcome),
        })
    }
}
