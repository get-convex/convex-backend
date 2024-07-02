mod in_process;
mod metrics;
#[allow(clippy::module_inception)]
mod searcher;
mod searchlight_knobs;
mod segment_cache;

pub use in_process::{
    InProcessSearcher,
    SearcherStub,
};
pub use searcher::{
    Bm25Stats,
    FieldDeletions,
    FragmentedTextStorageKeys,
    PostingListMatch,
    PostingListQuery,
    Searcher,
    SearcherImpl,
    SegmentTermMetadataFetcher,
    Term,
    TermDeletionsByField,
    TermValue,
    TokenMatch,
    TokenQuery,
};
pub use text_search::tracker::SegmentTermMetadata;
