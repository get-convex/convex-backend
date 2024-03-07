//! Runtime implementations for abstracting out core systems functionality. See
//! `[common::runtime::Runtime]`.
#![feature(binary_heap_drain_sorted)]
#![feature(lazy_cell)]
#![feature(never_type)]
pub mod prod;

#[cfg(any(test, feature = "testing"))]
pub use ::common::runtime::testing;
