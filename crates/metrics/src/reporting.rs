use std::collections::HashMap;

use prometheus::{
    core::Collector,
    Gauge,
    GaugeVec,
    IntCounter,
    IntCounterVec,
    VMHistogram,
    VMHistogramVec,
};

use crate::{
    labels::Labels,
    log_invalid_metric,
    MetricLabel,
};

pub fn log_counter(prometheus_counter: &IntCounter, increment: u64) {
    prometheus_counter.inc_by(increment);
}

pub fn log_counter_with_labels(
    prometheus_counter: &IntCounterVec,
    increment: u64,
    labels: Labels<'_>,
) {
    match prometheus_counter.get_metric_with(
        &labels
            .iter()
            .map(MetricLabel::split_key_value)
            .collect::<HashMap<_, _, ahash::RandomState>>(),
    ) {
        Ok(metric) => metric.inc_by(increment),
        Err(e) => {
            log_invalid_metric(get_desc(prometheus_counter), e);
        },
    }
}
// TODO: Callers should be allowed to use an `IntGauge` or `Gauge` as needed.
pub fn log_gauge(prometheus_gauge: &Gauge, value: f64) {
    prometheus_gauge.set(value);
}

pub fn log_gauge_with_labels(prometheus_gauge: &GaugeVec, value: f64, labels: Labels<'_>) {
    match prometheus_gauge.get_metric_with(
        &labels
            .iter()
            .map(MetricLabel::split_key_value)
            .collect::<HashMap<_, _, ahash::RandomState>>(),
    ) {
        Ok(metric) => metric.set(value),
        Err(e) => {
            log_invalid_metric(get_desc(prometheus_gauge), e);
        },
    }
}

pub fn log_distribution(prometheus_histogram: &VMHistogram, value: f64) {
    prometheus_histogram.observe(value);
}

pub fn log_distribution_with_labels(
    prometheus_histogram: &VMHistogramVec,
    value: f64,
    labels: Labels<'_>,
) {
    match prometheus_histogram.get_metric_with(
        &labels
            .iter()
            .map(MetricLabel::split_key_value)
            .collect::<HashMap<_, _, ahash::RandomState>>(),
    ) {
        Ok(metric) => metric.observe(value),
        Err(e) => {
            log_invalid_metric(get_desc(prometheus_histogram), e);
        },
    }
}

pub fn get_desc<M: Collector>(metric: &M) -> String {
    let unknown = "unknown".to_string();
    metric
        .desc()
        .first()
        .map(|d| d.fq_name.clone())
        .unwrap_or(unknown)
}
