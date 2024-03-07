pub mod compactor;
pub mod fast_forward;
pub mod flusher;
pub mod writer;

use common::{
    bootstrap_model::index::vector_index::{
        DeveloperVectorIndexConfig,
        VectorIndexState,
    },
    types::{
        IndexId,
        TabletIndexName,
    },
};
use value::ResolvedDocumentId;

use crate::index_workers::BuildReason;

pub(crate) struct IndexBuild {
    index_name: TabletIndexName,
    by_id: IndexId,
    developer_config: DeveloperVectorIndexConfig,
    metadata_id: ResolvedDocumentId,
    on_disk_state: VectorIndexState,
    build_reason: BuildReason,
}
