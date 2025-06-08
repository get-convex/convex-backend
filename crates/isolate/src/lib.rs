#![feature(const_type_name)]
#![feature(exclusive_wrapper)]
#![feature(try_blocks)]
#![feature(iterator_try_collect)]
#![feature(type_alias_impl_trait)]
#![feature(let_chains)]
#![feature(never_type)]
#![feature(assert_matches)]
#![feature(impl_trait_in_assoc_type)]
#![feature(round_char_boundary)]

mod array_buffer_allocator;
pub mod bundled_js;
pub mod client;
mod concurrency_limiter;
pub mod environment;
pub mod error;
mod execution_scope;
pub mod helpers;
mod http;
mod is_instance_of_error;
pub mod isolate;
pub mod isolate2;
pub mod isolate_worker;
pub mod metrics;
pub mod module_cache;
pub mod module_map;
mod ops;
mod request_scope;
pub mod strings;
mod termination;
#[cfg(test)]
mod tests;
mod timeout;
mod udf_runtime;

#[cfg(any(test, feature = "testing"))]
pub mod test_helpers;
pub use self::{
    client::{
        ActionCallbacks,
        ActionRequest,
        ActionRequestParams,
        IsolateClient,
        IsolateConfig,
        UdfCallback,
    },
    concurrency_limiter::{
        ConcurrencyLimiter,
        ConcurrencyPermit,
    },
    execution_scope::ExecutionScope,
    helpers::{
        deserialize_udf_custom_error,
        deserialize_udf_result,
        format_uncaught_error,
        UdfArgsJson,
    },
    isolate::IsolateHeapStats,
    metrics::{
        log_source_map_missing,
        log_source_map_token_lookup_failed,
    },
    request_scope::RequestScope,
    timeout::Timeout,
};
