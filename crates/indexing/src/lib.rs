#![feature(bound_as_ref)]
#![feature(coroutines)]
#![feature(once_cell_try)]
#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]
#![feature(iter_from_coroutine)]
#![feature(try_blocks)]

pub mod backend_in_memory_indexes;
pub mod index_registry;
pub mod interval;
mod metrics;

#[cfg(test)]
mod tests;
