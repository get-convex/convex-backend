//! Each module here represents a bootstrap table - a table required for the
//! low level database to start up properly.
//!
//! This mirrors the database/src/bootstrap_model directory.
//! It's preferable to have types colocated to the model in the database crate,
//! but some of these low level crates have types that are easier to keep in
//! `common` since they are used in so many places, so we mirror that over here
//! for ease of access to the metadata objects.
//!
//! We should strive to have future bootstrap tables have their metadata
//! colocated in database crate.
pub mod components;
pub mod index;
pub mod schema;
mod schema_metadata;
mod schema_state;
pub mod tables;
