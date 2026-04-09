use std::sync::Arc;

use common::runtime::Runtime;
use search::Searcher;
use storage::Storage;

use crate::{
    search_index_workers::search_compactor::{
        CompactionConfig,
        SearchIndexCompactor,
    },
    text_index_worker::{
        text_meta::TextSearchIndex,
        TextIndexMetadataWriter,
    },
    Database,
};

pub type TextIndexCompactor<RT> = SearchIndexCompactor<RT, TextSearchIndex>;

pub(crate) fn new_text_compactor<RT: Runtime>(
    database: Database<RT>,
    searcher: Arc<dyn Searcher>,
    search_storage: Arc<dyn Storage>,
    config: CompactionConfig,
    writer: TextIndexMetadataWriter<RT>,
) -> TextIndexCompactor<RT> {
    TextIndexCompactor::new(database, searcher, search_storage, config, writer)
}
