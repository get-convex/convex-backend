use anyhow::Context;
use common::identity::InertIdentity;
use pb::outcome::{
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
    HttpActionRequestHead,
    ValidatedHttpPath,
    ValidatedPathAndArgs,
};

/// A `UdfOutcome` represents a successful execution of a developer's function
/// by our V8 layer before it has had its returns validator run. It slightly
/// differs from `UdfExecution`, which is what we store in memory for logs.
#[derive(Clone)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
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
        path_and_args: Option<ValidatedPathAndArgs>,
        http_metadata: Option<(ValidatedHttpPath, HttpActionRequestHead)>,
        identity: InertIdentity,
    ) -> anyhow::Result<Self> {
        let outcome = outcome.ok_or_else(|| anyhow::anyhow!("Missing outcome"))?;
        match outcome {
            OutcomeProto::Query(outcome) => Ok(FunctionOutcome::Query(UdfOutcome::from_proto(
                outcome,
                path_and_args.context("Missing path and args")?,
                identity,
            )?)),
            OutcomeProto::Mutation(outcome) => {
                Ok(FunctionOutcome::Mutation(UdfOutcome::from_proto(
                    outcome,
                    path_and_args.context("Missing path and args")?,
                    identity,
                )?))
            },
            OutcomeProto::Action(outcome) => {
                Ok(FunctionOutcome::Action(ActionOutcome::from_proto(
                    outcome,
                    path_and_args.context("Missing path and args")?,
                    identity,
                )?))
            },
            OutcomeProto::HttpAction(outcome) => {
                let (http_path, http_request) = http_metadata.context("Missing http metadata")?;
                Ok(FunctionOutcome::HttpAction(HttpActionOutcome::from_proto(
                    outcome,
                    http_request,
                    http_path.npm_version().clone(),
                    identity,
                )?))
            },
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
            FunctionOutcome::HttpAction(outcome) => OutcomeProto::HttpAction(outcome.try_into()?),
        };
        Ok(FunctionOutcomeProto {
            outcome: Some(outcome),
        })
    }
}
