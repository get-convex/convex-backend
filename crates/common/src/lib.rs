//! Common code, types and libraries for interacting with the system.
#![feature(assert_matches)]
#![feature(binary_heap_drain_sorted)]
#![feature(const_for)]
#![feature(const_type_name)]
#![feature(coroutines)]
#![feature(iter_intersperse)]
#![feature(let_chains)]
#![feature(nonzero_ops)]
#![feature(try_blocks)]
#![feature(type_alias_impl_trait)]
#![feature(bound_as_ref)]
#![feature(iter_from_coroutine)]
#![feature(iterator_try_collect)]
#![feature(error_iter)]
#![feature(impl_trait_in_assoc_type)]
#![feature(round_char_boundary)]
#![feature(never_type)]
#![feature(error_generic_member_access)]
#![feature(str_split_remainder)]
#![feature(duration_constructors)]

pub mod async_compat;
pub mod auth;
pub mod backoff;
pub mod bootstrap_model;
pub mod bounds;
pub mod client_pool;
pub mod codel_queue;
pub mod comparators;
pub mod components;
pub mod deleted_bitset;
pub mod document;
pub mod errors;
pub mod execution_context;
pub mod ext;
pub mod fastrace_helpers;
pub mod floating_point;
pub mod grpc;
pub mod heap_size;
pub mod http;
pub mod id_tracker;
pub mod identifier;
pub mod identity;
pub mod index;
pub mod interval;
pub mod is_canceled;
pub mod json;
pub mod json_schemas;
pub mod knobs;
pub mod log_lines;
pub mod log_streaming;
pub mod metrics;
pub mod numeric;
pub mod paths;
pub mod pause;
pub mod persistence;
pub mod persistence_helpers;
pub mod pii;
pub mod pool_stats;
pub mod query;
pub mod query_journal;
pub mod retriable_stream;
pub mod runtime;
pub mod schemas;
pub mod sha256;
pub mod shapes;
pub mod shutdown;
pub mod sync;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
pub mod tracing;
pub mod types;
pub mod utils;
pub mod virtual_system_mapping;
pub use value;
pub mod bounded_thread_pool;
pub mod try_chunks;
pub mod version;
pub mod ws;

pub use execution_context::RequestId;
pub use tokio;
#[cfg(any(test, feature = "testing"))]
pub use value::assert_obj;
pub use value::{
    obj,
    val,
};
