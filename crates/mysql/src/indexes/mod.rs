//! Index entry storage formats. Each submodule implements the write,
//! retention-load, and retention-delete data paths for one on-disk layout of
//! the index tables.

pub(crate) mod single_table;
