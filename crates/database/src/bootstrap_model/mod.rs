//! Each module here represents a bootstrap table - a table required for the
//! low level database to start up properly.
//!
//! Higher level tables belong in the model crate, layered above the database.
pub mod defaults;
pub mod index;
pub mod index_workers;
pub mod schema;
pub mod table;
pub mod virtual_tables;
