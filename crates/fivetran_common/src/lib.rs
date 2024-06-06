#![feature(impl_trait_in_assoc_type)]

pub mod config;
pub mod fivetran_sdk;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
