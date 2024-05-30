use crate::{
    index_workers::writer::SearchIndexMetadataWriter,
    text_index_worker::text_meta::TextSearchIndex,
};

pub mod compactor;
pub mod fast_forward;
pub mod flusher;
pub mod flusher2;
mod text_meta;

pub type TextIndexMetadataWriter<RT> = SearchIndexMetadataWriter<RT, TextSearchIndex>;
