use crate::{
    search_index_workers::writer::SearchIndexMetadataWriter,
    text_index_worker::text_meta::TextSearchIndex,
};

pub mod compactor;
pub mod fast_forward;
pub mod flusher;
mod text_meta;

pub type TextIndexMetadataWriter<RT> = SearchIndexMetadataWriter<RT, TextSearchIndex>;
pub use text_meta::BuildTextIndexArgs;
