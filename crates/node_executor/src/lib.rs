#![feature(lazy_cell)]
#![feature(assert_matches)]
#![feature(never_type)]
#![feature(let_chains)]
#![feature(unwrap_infallible)]
#![feature(iterator_try_collect)]
#![feature(stmt_expr_attributes)]
#![feature(try_blocks)]

mod executor;
pub mod local;
mod metrics;
pub mod source_package;

pub use crate::executor::{
    error_response_json,
    parse_streamed_response,
    Actions,
    AnalyzeRequest,
    AnalyzeResponse,
    BuildDepsRequest,
    ExecuteRequest,
    ExecutorRequest,
    InvokeResponse,
    NodeActionOutcome,
    NodeExecutor,
    Package,
    ResponsePart,
    SourcePackage,
    EXECUTE_TIMEOUT_RESPONSE_JSON,
};
