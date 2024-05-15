#![feature(const_mut_refs)]
#![feature(const_type_name)]
#![feature(lazy_cell)]
#![feature(async_closure)]
#![feature(try_blocks)]
#![feature(const_trait_impl)]
#![feature(iterator_try_collect)]
#![feature(type_alias_impl_trait)]
#![feature(let_chains)]
#![feature(never_type)]
#![feature(assert_matches)]
#![feature(impl_trait_in_assoc_type)]
#![feature(arc_unwrap_or_clone)]
#![feature(round_char_boundary)]

pub mod bundled_js;
pub mod client;
mod concurrency_limiter;
pub mod environment;
pub mod error;
mod execution_scope;
pub mod helpers;
mod http;
mod http_action;
mod is_instance_of_error;
pub mod isolate;
pub mod isolate2;
pub mod metrics;
pub mod module_map;
mod ops;
mod request_scope;
pub mod strings;
mod termination;
#[cfg(test)]
mod tests;
mod timeout;

#[cfg(any(test, feature = "testing"))]
pub mod test_helpers;
#[cfg(any(test, feature = "testing"))]
pub use self::http_action::HttpActionResponse;
pub use self::{
    client::{
        ActionCallbacks,
        ActionRequest,
        ActionRequestParams,
        BackendIsolateWorker,
        FunctionResult,
        IsolateClient,
        IsolateConfig,
    },
    concurrency_limiter::ConcurrencyLimiter,
    environment::{
        action::{
            outcome::{
                ActionOutcome,
                HttpActionOutcome,
            },
            HttpActionResult,
        },
        auth_config::AuthConfig,
        helpers::{
            validation::{
                validate_schedule_args,
                ValidatedHttpPath,
                ValidatedPathAndArgs,
            },
            FunctionOutcome,
            JsonPackedValue,
            SyscallStats,
            SyscallTrace,
        },
        udf::{
            outcome::UdfOutcome,
            CONVEX_ORIGIN,
            CONVEX_SITE,
        },
    },
    helpers::{
        deserialize_udf_custom_error,
        deserialize_udf_result,
        format_uncaught_error,
        parse_udf_args,
        serialize_udf_args,
        UdfArgsJson,
    },
    http_action::{
        HttpActionRequest,
        HttpActionRequestHead,
        HttpActionResponseHead,
        HttpActionResponsePart,
        HttpActionResponseStreamer,
        HTTP_ACTION_BODY_LIMIT,
    },
    isolate::IsolateHeapStats,
    metrics::{
        log_source_map_missing,
        log_source_map_token_lookup_failed,
    },
};
