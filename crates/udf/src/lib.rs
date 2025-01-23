#![feature(impl_trait_in_assoc_type)]
#![feature(iterator_try_collect)]

mod action_outcome;
mod client;
pub mod environment;
mod function_outcome;
pub mod helpers;
mod http_action;
mod syscall_stats;
mod syscall_trace;
mod udf_outcome;
pub mod validation;

#[cfg(any(test, feature = "testing"))]
pub use crate::http_action::HttpActionResponse;
pub use crate::{
    action_outcome::{
        ActionOutcome,
        HttpActionOutcome,
        HttpActionResult,
    },
    client::{
        EvaluateAppDefinitionsResult,
        FunctionResult,
    },
    function_outcome::FunctionOutcome,
    http_action::{
        HttpActionRequest,
        HttpActionRequestHead,
        HttpActionResponseHead,
        HttpActionResponsePart,
        HttpActionResponseStreamer,
        HTTP_ACTION_BODY_LIMIT,
    },
    syscall_stats::SyscallStats,
    syscall_trace::SyscallTrace,
    udf_outcome::UdfOutcome,
};
