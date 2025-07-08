//! Each module here represents a bootstrap table - a table required for the
//! low level database to start up properly.
//!
//! Higher level tables belong in the model crate, layered above the database.
pub mod components;
pub mod defaults;
pub mod import_facing;
pub mod index;
pub mod index_backfills;
pub mod index_workers;
pub mod schema;
pub mod system_metadata;
pub mod table;
pub mod user_facing;

#[cfg(any(test, feature = "testing"))]
pub mod test_facing;
