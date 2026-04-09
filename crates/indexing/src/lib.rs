#![feature(bound_as_ref)]
#![feature(coroutines)]
#![feature(once_cell_try)]
#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]
#![feature(iter_from_coroutine)]
#![feature(try_blocks)]
#![feature(try_blocks_heterogeneous)]
#![feature(anonymous_lifetime_in_impl_trait)]

pub mod backend_in_memory_indexes;
pub mod index_cache;
pub mod index_registry;
pub mod metrics;
