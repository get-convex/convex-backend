use std::collections::HashMap;

use common::types::SearchIndexMetricLabels;
use metrics::{
    add_to_gauge_with_labels,
    log_counter,
    log_distribution_with_labels,
    log_gauge,
    log_gauge_with_labels,
    log_invalid_metric,
    register_convex_counter,
    register_convex_gauge,
    register_convex_histogram,
    subtract_from_gauge_with_labels,
    CancelableTimer,
    MetricLabel,
    StatusTimer,
    STATUS_LABEL,
};
use once_cell::sync::Lazy;
use parking_lot::Mutex;

use crate::{
    metrics::SEARCH_FILE_TYPE,
    searcher::ARCHIVE_METRIC_EMIT_THRESHOLD_FRACTION,
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

const INDEX_ID_LABEL: &str = "index_id";
const CONVEX_DEPLOYMENT_LABEL: &str = "convex_deployment";
register_convex_gauge!(
    SEARCHLIGHT_ARCHIVE_EXTRACT_BYTES_BY_INDEX_BYTES,
    "Total bytes extracted for an index across the archive cache",
    &[SEARCH_FILE_TYPE, INDEX_ID_LABEL, CONVEX_DEPLOYMENT_LABEL],
);
register_convex_gauge!(
    SEARCHLIGHT_ARCHIVE_EXTRACT_BYTES_BY_DEPLOYMENT_BYTES,
    "Total bytes extracted across the archive cache for a deployment",
    &[SEARCH_FILE_TYPE, CONVEX_DEPLOYMENT_LABEL],
);

#[derive(Hash, PartialEq, Eq, Clone)]
struct IndexGaugeKey {
    search_file_type: SearchFileType,
    index_id: Option<common::types::IndexId>,
    convex_deployment: Option<String>,
}

#[derive(Hash, PartialEq, Eq, Clone)]
struct DeploymentGaugeKey {
    search_file_type: SearchFileType,
    convex_deployment: Option<String>,
}

static INDEX_GAUGE_VALUES: Lazy<Mutex<std::collections::HashMap<IndexGaugeKey, i64>>> =
    Lazy::new(|| Mutex::new(std::collections::HashMap::new()));
static DEPLOYMENT_GAUGE_VALUES: Lazy<Mutex<std::collections::HashMap<DeploymentGaugeKey, i64>>> =
    Lazy::new(|| Mutex::new(std::collections::HashMap::new()));

fn update_gauge<K: Eq + std::hash::Hash + Clone>(
    current: &mut std::collections::HashMap<K, i64>,
    key: K,
    delta: i64,
) -> i64 {
    let entry = current.entry(key.clone()).or_insert(0);
    *entry += delta;
    if *entry == 0 {
        current.remove(&key);
        0
    } else {
        *entry
    }
}

fn total_cache_size_for_filetype(search_file_type: SearchFileType) -> f64 {
    let labels_vec = vec![search_file_type.metric_label()];
    let labels_map: HashMap<_, _> = labels_vec
        .iter()
        .map(MetricLabel::split_key_value)
        .collect();
    match SEARCHLIGHT_CACHE_USED_BY_TYPE_BYTES.get_metric_with(&labels_map) {
        Ok(metric) => metric.get(),
        Err(e) => {
            log_invalid_metric(metrics::get_desc(&*SEARCHLIGHT_CACHE_USED_BY_TYPE_BYTES), e);
            0.0
        },
    }
}

fn should_emit(value: i64, search_file_type: SearchFileType) -> bool {
    let total_cache_bytes = total_cache_size_for_filetype(search_file_type);
    let threshold = total_cache_bytes * *ARCHIVE_METRIC_EMIT_THRESHOLD_FRACTION;
    (value.unsigned_abs() as f64) > threshold
}

pub fn adjust_archive_bytes_for_index(
    bytes_delta: i64,
    search_file_type: SearchFileType,
    labels: SearchIndexMetricLabels<'_>,
) {
    let convex_deployment = labels
        .convex_deployment()
        .unwrap_or("unknown_deployment")
        .to_string();
    let index_id = labels.index_id();
    let index_value = index_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| "unknown_index".to_string());
    let search_file_type_label = search_file_type.metric_label();
    let search_file_type_value = search_file_type_label.value.clone();
    let mut gauges = INDEX_GAUGE_VALUES.lock();
    let new_total = update_gauge(
        &mut gauges,
        IndexGaugeKey {
            search_file_type,
            index_id,
            convex_deployment: Some(convex_deployment.clone()),
        },
        bytes_delta,
    );
    let labels_vec = vec![
        search_file_type_label.clone(),
        metrics::StaticMetricLabel::new(INDEX_ID_LABEL, index_value.clone()),
        metrics::StaticMetricLabel::new(CONVEX_DEPLOYMENT_LABEL, convex_deployment.clone()),
    ];
    if should_emit(new_total, search_file_type) {
        log_gauge_with_labels(
            &SEARCHLIGHT_ARCHIVE_EXTRACT_BYTES_BY_INDEX_BYTES,
            new_total as f64,
            labels_vec,
        );
    } else {
        let _ = SEARCHLIGHT_ARCHIVE_EXTRACT_BYTES_BY_INDEX_BYTES.remove_label_values(&[
            search_file_type_value.as_ref(),
            index_value.as_str(),
            convex_deployment.as_str(),
        ]);
    }
    adjust_archive_bytes_for_deployment(bytes_delta, search_file_type, &convex_deployment);
}

pub fn adjust_archive_bytes_for_deployment(
    bytes_delta: i64,
    search_file_type: SearchFileType,
    convex_deployment: &str,
) {
    let search_file_type_label = search_file_type.metric_label();
    let search_file_type_value = search_file_type_label.value.clone();
    let mut gauges = DEPLOYMENT_GAUGE_VALUES.lock();
    let new_total = update_gauge(
        &mut gauges,
        DeploymentGaugeKey {
            search_file_type,
            convex_deployment: Some(convex_deployment.to_string()),
        },
        bytes_delta,
    );
    let labels_vec = vec![
        search_file_type_label.clone(),
        metrics::StaticMetricLabel::new(CONVEX_DEPLOYMENT_LABEL, convex_deployment.to_owned()),
    ];
    if should_emit(new_total, search_file_type) {
        log_gauge_with_labels(
            &SEARCHLIGHT_ARCHIVE_EXTRACT_BYTES_BY_DEPLOYMENT_BYTES,
            new_total as f64,
            labels_vec,
        );
    } else {
        let _ = SEARCHLIGHT_ARCHIVE_EXTRACT_BYTES_BY_DEPLOYMENT_BYTES
            .remove_label_values(&[search_file_type_value.as_ref(), convex_deployment]);
    }
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

register_convex_gauge!(
    SEARCHLIGHT_CACHE_USED_BY_TYPE_BYTES,
    "Number of bytes used on disk by file type",
    &[SEARCH_FILE_TYPE]
);
pub fn add_bytes_by_file_type(search_file_type: SearchFileType, size: u64) {
    add_to_gauge_with_labels(
        &SEARCHLIGHT_CACHE_USED_BY_TYPE_BYTES,
        size as f64,
        vec![search_file_type.metric_label()],
    );
}
pub fn subtract_bytes_by_file_type(search_file_type: SearchFileType, size: u64) {
    subtract_from_gauge_with_labels(
        &SEARCHLIGHT_CACHE_USED_BY_TYPE_BYTES,
        size as f64,
        vec![search_file_type.metric_label()],
    );
}
