pub mod compactor;
pub mod fast_forward;
pub mod flusher;
pub mod writer;

use common::types::{
    IndexId,
    TabletIndexName,
};
use value::ResolvedDocumentId;

use crate::index_workers::{
    index_meta::{
        SearchIndex,
        SearchIndexConfig,
    },
    BuildReason,
};

pub(crate) struct IndexBuild<T: SearchIndex> {
    index_name: TabletIndexName,
    index_id: IndexId,
    by_id: IndexId,
    metadata_id: ResolvedDocumentId,
    index_config: SearchIndexConfig<T>,
    build_reason: BuildReason,
}
