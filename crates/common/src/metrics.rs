use std::{
    net::SocketAddr,
    time::Duration,
};

use futures::{
    future::{
        BoxFuture,
        Either,
    },
    pin_mut,
    FutureExt,
};
use metrics::{
    log_counter,
    log_counter_with_labels,
    log_distribution,
    log_gauge,
    register_convex_counter,
    register_convex_gauge,
    register_convex_histogram,
    IntoLabel,
    StaticMetricLabel,
    Timer,
    CONVEX_METRICS_REGISTRY,
};
use prometheus::{
    VMHistogram,
    VMHistogramVec,
};
use sync_types::backoff::Backoff;

use crate::runtime::{
    Runtime,
    SpawnHandle,
};

register_convex_counter!(
    COMMON_UNDEFINED_FILTER_TOTAL,
    "Count of undefined JsonValues filtered"
);
pub fn log_undefined_filter() {
    log_counter(&COMMON_UNDEFINED_FILTER_TOTAL, 1);
}

register_convex_gauge!(COMMON_CODEL_QUEUE_LENGTH_TOTAL, "Length of the CoDel queue");
pub fn log_codel_queue_size(size: usize) {
    log_gauge(&COMMON_CODEL_QUEUE_LENGTH_TOTAL, size as f64)
}

register_convex_gauge!(
    COMMON_CODEL_QUEUE_OVERLOADED_TOTAL,
    "1 if the CoDel queue is overloaded, 0 otherwise"
);
pub fn log_codel_queue_overloaded(overloaded: bool) {
    log_gauge(
        &COMMON_CODEL_QUEUE_OVERLOADED_TOTAL,
        if overloaded { 1.0 } else { 0.0 },
    )
}

// static $metric: LazyLock<IntCounter> = LazyLock::new(|| {
// register_int_counter_with_registry!(&*$metricname, $help,
// CONVEX_METRICS_REGISTRY).unwrap()}); ==>> register_convex_counter!($metric,
// $help);
register_convex_histogram!(
    COMMON_CODEL_QUEUE_TIME_SINCE_EMPTY_SECONDS,
    "Time since the CoDel queue was empty"
);

pub fn log_codel_queue_time_since_empty(duration: Duration) {
    log_distribution(
        &COMMON_CODEL_QUEUE_TIME_SINCE_EMPTY_SECONDS,
        duration.as_secs_f64(),
    )
}

register_convex_counter!(
    CHECKED_INDEX_EXPIRATION_DOCUMENTS,
    "Count of documents checked for index expiration",
    &["expired", "reason"]
);
pub fn log_index_expiration_checked(expired: bool, reason: &'static str) {
    log_counter_with_labels(
        &CHECKED_INDEX_EXPIRATION_DOCUMENTS,
        1,
        vec![
            StaticMetricLabel::new("expired", expired.as_label()),
            StaticMetricLabel::new("reason", reason),
        ],
    );
}

register_convex_counter!(
    CLIENT_VERSION_UNSUPPORTED_TOTAL,
    "Count of requests with an unsupported client version",
    &["version"]
);
pub fn log_client_version_unsupported(version: String) {
    log_counter_with_labels(
        &CLIENT_VERSION_UNSUPPORTED_TOTAL,
        1,
        vec![StaticMetricLabel::new("version", version)],
    );
}

register_convex_histogram!(
    STATIC_REPEATABLE_TS_SECONDS,
    "Time taken for a timestamp to be repeatable",
    &["recent"]
);
pub fn static_repeatable_ts_timer(is_recent: bool) -> Timer<VMHistogramVec> {
    let mut timer = Timer::new_with_labels(&STATIC_REPEATABLE_TS_SECONDS);
    timer.add_label(StaticMetricLabel::new(
        "recent",
        if is_recent { "recent" } else { "at_ts" },
    ));
    timer
}

register_convex_counter!(ERRORS_REPORTED_TOTAL, "Count of errors reported", &["type"]);
pub fn log_errors_reported_total(tag: StaticMetricLabel) {
    log_counter_with_labels(&ERRORS_REPORTED_TOTAL, 1, vec![tag]);
}

pub type FlushMetrics<RT: Runtime> = impl FnOnce() -> BoxFuture<'static, ()>;

pub fn register_prometheus_exporter<RT: Runtime>(
    rt: RT,
    bind_addr: SocketAddr,
) -> (Box<dyn SpawnHandle>, FlushMetrics<RT>) {
    let rt_ = rt.clone();
    let handle = rt.clone().spawn("prometheus_exporter", async move {
        let mut backoff = Backoff::new(Duration::from_millis(10), Duration::from_secs(10));
        while let Err(e) = prometheus_hyper::Server::run(
            &*CONVEX_METRICS_REGISTRY,
            bind_addr,
            futures::future::pending(),
        )
        .await
        {
            let delay = backoff.fail(&mut rt.rng());
            tracing::error!(
                "Prometheus exporter server failed with error {e:?}, restarting after {}ms delay",
                delay.as_millis()
            );
            continue;
        }
    });
    let flush = || {
        async move {
            // Prometheus scrapes metrics every 30s.
            let shutdown = tokio::signal::ctrl_c();
            let flush_fut = rt_.wait(Duration::from_secs(60));
            pin_mut!(shutdown);
            pin_mut!(flush_fut);
            tracing::info!("Flushing metrics (60s)... Ctrl-C to skip");
            match futures::future::select(shutdown, flush_fut).await {
                Either::Left(_) => tracing::info!("Got another ctrl-C, shutting down"),
                Either::Right(_) => tracing::info!("Finished flushing metrics"),
            }
        }
        .boxed()
    };
    (handle, flush)
}

register_convex_histogram!(LOAD_ID_TRACKER_SECONDS, "Time to load IdTracker in seconds");
pub fn load_id_tracker_timer() -> Timer<VMHistogram> {
    Timer::new(&LOAD_ID_TRACKER_SECONDS)
}

register_convex_histogram!(ID_TRACKER_SIZE_BYTES, "IdTracker file size");
pub fn log_id_tracker_size(size: usize) {
    log_distribution(&ID_TRACKER_SIZE_BYTES, size as f64);
}
