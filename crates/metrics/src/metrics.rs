//! Common functions for metrics logging.
//!
//! We follow [Prometheus's conventions](https://prometheus.io/docs/practices/naming/) for metrics
//! names intersected with [Datadog's
//! requirements](https://docs.datadoghq.com/metrics/custom_metrics/). In particular,
//!
//! 1. Metrics may only contain alphanumerics and underscores.
//! 2. Metrics are automatically prefixed with `SERVICE_NAME`.
//! 3. Suffix metrics with their units (e.g. `_seconds`, `_bytes`, `_total`).
//! See `ALLOWED_SUFFIXES` for more detail. 4. Use seconds for time and bytes
//! for data. Use `_total` for unit-less counts.
//!
//! We also have a few conventions for instrumenting code within our crates.
//! 1. All metrics code goes in a `metrics` module. The interface to this module
//! should be high level (e.g. "this event happened") rather than logging an
//! `f64` to a particular metric name. 2. All metrics names and labels are
//! constants/string literals in the metrics module.
use std::{
    borrow::Cow,
    collections::HashSet,
    env,
    ops::Deref,
    sync::LazyLock,
};

use parking_lot::RwLock;
use prometheus::Registry;

use crate::{
    log_counter_with_labels,
    log_gauge,
    register_convex_counter,
    register_convex_gauge,
    StaticMetricLabel,
};

const ALLOWED_SUFFIXES: &[&str] = &[
    // Always use `_seconds` for time.
    "_seconds",
    // Always use `_bytes` for data lengths.
    "_bytes",
    // Database units.
    "_documents",
    "_rows",
    "_queries",
    "_statements",
    "_commits",
    "_tables",
    "_transactions",
    // Networking units.
    "_connections",
    "_requests",
    "_timeouts",
    "_sessions",
    // Caching units.
    "_hits",
    "_misses",
    "_evictions",
    // System units.
    "_threads",
    // General units.
    "_errors",
    "_reads",
    "_writes",
    "_operations",
    "_updates",
    // Use `_total` as a generic unit-less count that doesn't fit into a unit above.
    "_total",
    // Use `_info` as a generic unit-less gauge that doesn't fit into a unit above.
    "_info",
];

/// Use executable name to identify service name)
pub static SERVICE_NAME: LazyLock<String> = LazyLock::new(|| {
    let path = std::env::current_exe().expect("Couldn't find exe name");
    path.file_name()
        .expect("Path was empty")
        .to_str()
        .expect("Not valid unicode")
        .replace('-', "_")
});

pub static CONVEX_METRICS_REGISTRY: LazyLock<Registry> = LazyLock::new(|| {
    let labels = env::var("CONVEX_SITE").ok().map(|instance_name| {
        [("instance_name".to_owned(), instance_name)]
            .into_iter()
            .collect()
    });
    Registry::new_custom(Some(SERVICE_NAME.clone()), labels)
        .expect("Failed to initialize Prometheus metrics registry")
});

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct MetricName(Cow<'static, str>);

impl MetricName {
    pub const fn new(name: &'static str) -> Self {
        validate_metric_name(name);
        Self(Cow::Borrowed(name))
    }
}

impl Deref for MetricName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0[..]
    }
}

const fn ends_with(s: &[u8], suffix: &[u8]) -> bool {
    if s.len() < suffix.len() {
        return false;
    }
    let s_base = s.len() - suffix.len();
    let mut i = 0;
    while i < suffix.len() {
        if s[s_base + i] != suffix[i] {
            return false;
        }
        i += 1;
    }
    true
}

// TODO: Write this with more standard Rust as more `const fn` functionality is
// stabilized.
const fn validate_metric_name(name: &str) {
    let name_bytes = name.as_bytes();

    // Check that all the characters are valid.
    let mut i = 0;
    while i < name_bytes.len() {
        let c = name_bytes[i];
        let is_upper = 65 <= c && c <= 90;
        let is_lower = 97 <= c && c <= 122;
        let is_numeric = 48 <= c && c <= 57;
        let is_underscore = c == 95;
        if !(is_upper || is_lower || is_numeric || is_underscore) {
            panic!("Metric names can only contain alphanumeric characters and underscores");
        }
        i += 1;
    }

    let mut i = 0;
    let mut found_suffix = false;
    while i < ALLOWED_SUFFIXES.len() {
        if ends_with(name_bytes, ALLOWED_SUFFIXES[i].as_bytes()) {
            found_suffix = true;
            break;
        }
        i += 1;
    }
    if !found_suffix {
        panic!(
            "Metric names must end with their units as a suffix (e.g. `_seconds`, `_bytes`, \
             `_total`)"
        );
    }
}

// Use a macro to force metric name validation to happen at compile time.
#[macro_export]
macro_rules! metric_name {
    ($name: expr) => {{
        use $crate::MetricName;
        const METRIC_NAME: MetricName = MetricName::new($name);
        METRIC_NAME
    }};
}

impl Deref for MetricHelp {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetricHelp(&'static str);

impl MetricHelp {
    pub const fn new(help_str: &'static str) -> Self {
        if help_str.is_empty() {
            panic!("Metric help strings must be nonempty");
        }
        Self(help_str)
    }
}

#[macro_export]
macro_rules! metric_help {
    ($help: literal) => {{
        use $crate::MetricHelp;
        const METRIC_HELP: MetricHelp = MetricHelp::new($help);
        METRIC_HELP
    }};
}

register_convex_counter!(
    INVALID_METRIC_TOTAL,
    "Count of metrics that failed to be reported",
    &["metric_name"]
);

// This is used to make sure we only report a metric failure once to Sentry, as
// it could easily grow out of proportion if we push a bad metric.
static METRICS_ERROR_ONCE: LazyLock<RwLock<HashSet<String>>> = LazyLock::new(Default::default);
pub fn log_invalid_metric(name: String, error: prometheus::Error) {
    log_counter_with_labels(
        &INVALID_METRIC_TOTAL,
        1,
        vec![StaticMetricLabel::new("metric_name", name.clone())],
    );
    if METRICS_ERROR_ONCE.read().contains(&name) {
        return;
    }
    if METRICS_ERROR_ONCE.write().insert(name.clone()) {
        let msg = format!("Failed to record metric {name:?}: {error}");
        if cfg!(any(test, feature = "testing")) {
            panic!("{msg}");
        }
        let err = anyhow::anyhow!(error).context(msg);
        tracing::error!("{:?}", err);
        #[allow(clippy::disallowed_methods)]
        sentry::integrations::anyhow::capture_anyhow(&err);
    }
}

register_convex_gauge!(
    DATABASE_SEARCH_IN_MEMORY_BYTES,
    "Number of bytes in memory for search indexes"
);
pub fn log_search_in_memory_size(bytes: usize) {
    log_gauge(&DATABASE_SEARCH_IN_MEMORY_BYTES, bytes as f64);
}

register_convex_gauge!(
    DATABASE_VECTOR_IN_MEMORY_BYTES,
    "Number of bytes in memory for vector indexes"
);
pub fn log_vector_in_memory_size(bytes: usize) {
    log_gauge(&DATABASE_VECTOR_IN_MEMORY_BYTES, bytes as f64);
}
