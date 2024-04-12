use metrics::{
    log_counter,
    log_distribution_with_labels,
    log_gauge,
    register_convex_counter,
    register_convex_gauge,
    register_convex_histogram,
    CancelableTimer,
    StatusTimer,
    STATUS_LABEL,
};

use crate::{
    metrics::SEARCH_FILE_TYPE,
    SearchFileType,
};

register_convex_histogram!(
    SEARCHLIGHT_ARCHIVE_FETCH_SECONDS,
    "Time taken to fetch an archive from S3",
    &STATUS_LABEL
);
pub fn archive_fetch_timer() -> StatusTimer {
    StatusTimer::new(&SEARCHLIGHT_ARCHIVE_FETCH_SECONDS)
}

register_convex_histogram!(
    SEARCHLIGHT_ARCHIVE_BYTES,
    "Bytes used on disk after extracting an archive ",
    &[SEARCH_FILE_TYPE],
);
pub fn finish_archive_fetch(timer: StatusTimer, bytes_used: u64, search_file_type: SearchFileType) {
    log_distribution_with_labels(
        &SEARCHLIGHT_ARCHIVE_BYTES,
        bytes_used as f64,
        vec![search_file_type.metric_label()],
    );
    timer.finish();
}
register_convex_histogram!(
    SEARCHLIGHT_ARCHIVE_GET_SECONDS,
    "Time taken for Searchlight to fetch an archive from S3",
    &[STATUS_LABEL[0], SEARCH_FILE_TYPE],
);
pub fn archive_get_timer(search_file_type: SearchFileType) -> CancelableTimer {
    let mut timer = CancelableTimer::new(&SEARCHLIGHT_ARCHIVE_GET_SECONDS);
    timer.add_label(search_file_type.metric_label());
    timer
}

register_convex_counter!(
    SEARCHLIGHT_ARCHIVE_CACHE_FETCH_TIMEOUT_TOTAL,
    "Count of requests which timed out fetching an archive"
);
pub fn log_cache_fetch_timeout() {
    log_counter(&SEARCHLIGHT_ARCHIVE_CACHE_FETCH_TIMEOUT_TOTAL, 1);
}

register_convex_histogram!(
    SEARCHLIGHT_ARCHIVE_CACHE_UNTAR_SECONDS,
    "The amount of time it takes to untar a qdrant segment in the archive cache",
    &STATUS_LABEL,
);
pub fn archive_untar_timer() -> StatusTimer {
    StatusTimer::new(&SEARCHLIGHT_ARCHIVE_CACHE_UNTAR_SECONDS)
}

register_convex_histogram!(
    SEARCHLIGHT_ARCHIVE_CACHE_EXTRACT_ARCHIVE_SECONDS,
    "The amount of time it takes to untar a qdrant segment in the archive cache",
    &STATUS_LABEL,
);
pub fn extract_archive_timer() -> StatusTimer {
    StatusTimer::new(&SEARCHLIGHT_ARCHIVE_CACHE_EXTRACT_ARCHIVE_SECONDS)
}

register_convex_gauge!(SEARCHLIGHT_USED_BYTES, "Number of bytes used on disk");
register_convex_gauge!(SEARCHLIGHT_MAX_BYTES, "Maxiumum size on disk permitted");
pub fn log_bytes_used(used: u64, max: u64) {
    log_gauge(&SEARCHLIGHT_USED_BYTES, used as f64);
    log_gauge(&SEARCHLIGHT_MAX_BYTES, max as f64);
}
