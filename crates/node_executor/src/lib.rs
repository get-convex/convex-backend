#![feature(assert_matches)]
#![feature(never_type)]
#![feature(let_chains)]
#![feature(unwrap_infallible)]
#![feature(iterator_try_collect)]
#![feature(stmt_expr_attributes)]
#![feature(try_blocks)]
#![feature(slice_split_once)]
#![feature(coroutines)]

mod executor;
pub mod local;
mod metrics;
pub mod noop;
pub mod source_package;

pub use crate::executor::{
    error_response_json,
    handle_node_executor_stream,
    Actions,
    AnalyzeRequest,
    AnalyzeResponse,
    BuildDepsRequest,
    ExecuteRequest,
    ExecutorRequest,
    InvokeResponse,
    NodeActionOutcome,
    NodeExecutor,
    NodeExecutorStreamPart,
    Package,
    ResponsePart,
    SourcePackage,
    EXECUTE_TIMEOUT_RESPONSE_JSON,
};
