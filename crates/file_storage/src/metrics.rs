use metrics::{
    log_distribution_with_labels,
    register_convex_histogram,
    MetricLabel,
    StatusTimer,
    STATUS_LABEL,
};

register_convex_histogram!(
    STORE_FILE_TOTAL_SECONDS,
    "Duration of persisting a file to storage",
    &STATUS_LABEL
);
pub fn store_file_timer() -> StatusTimer {
    StatusTimer::new(&STORE_FILE_TOTAL_SECONDS)
}

const GET_FILE_TYPE_LABEL: &str = "type";

#[derive(Clone, Copy)]
pub(crate) enum GetFileType {
    /// If a range is not specified in the request
    All,
    // If a range is specified in the request, even if the range is 0-
    Range,
}

impl GetFileType {
    fn tag(&self) -> MetricLabel {
        match self {
            GetFileType::All => MetricLabel::new(GET_FILE_TYPE_LABEL, "all"),
            GetFileType::Range => MetricLabel::new(GET_FILE_TYPE_LABEL, "range"),
        }
    }
}

register_convex_histogram!(
    GET_FILE_CHUNK_SIZE_BYTES,
    "The size of each chunk of data we return in file streams in bytes",
    &[GET_FILE_TYPE_LABEL],
);
pub fn log_get_file_chunk_size(size_bytes: u64, get_file_type: GetFileType) {
    log_distribution_with_labels(
        &GET_FILE_CHUNK_SIZE_BYTES,
        size_bytes as f64,
        vec![get_file_type.tag()],
    );
}
