//! This crate exists to duplicate logic from the model crate
//! so that we can evolve model over time, while freezing the old version of the
//! model code in time at the time of the migration. This allows migrations to
//! continue to work long after the model has been updated.

#![feature(coroutines)]
#![feature(try_blocks)]
#![feature(impl_trait_in_assoc_type)]

pub mod migr_119;
