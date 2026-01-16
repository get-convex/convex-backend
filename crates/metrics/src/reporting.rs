use std::collections::HashMap;

use prometheus::{
    core::Collector,
    Gauge,
    GaugeVec,
    IntCounter,
    IntCounterVec,
    IntGauge,
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

pub fn add_to_gauge_with_labels(prometheus_gauge: &GaugeVec, delta: f64, labels: Labels<'_>) {
    match prometheus_gauge.get_metric_with(
        &labels
            .iter()
            .map(MetricLabel::split_key_value)
            .collect::<HashMap<_, _, ahash::RandomState>>(),
    ) {
        Ok(metric) => metric.add(delta),
        Err(e) => {
            log_invalid_metric(get_desc(prometheus_gauge), e);
        },
    }
}

pub fn subtract_from_gauge_with_labels(
    prometheus_gauge: &GaugeVec,
    delta: f64,
    labels: Labels<'_>,
) {
    match prometheus_gauge.get_metric_with(
        &labels
            .iter()
            .map(MetricLabel::split_key_value)
            .collect::<HashMap<_, _, ahash::RandomState>>(),
    ) {
        Ok(metric) => metric.sub(delta),
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

/// Slices up an `IntGauge` into many independently-`set`table pieces.
/// The final gauge value is the sum of all the live `Subgauge`s' values.
///
/// When using this, don't call `set` directly on the underlying gauge.
pub struct Subgauge {
    gauge: IntGauge,
    value: i64,
}
impl Subgauge {
    pub fn new(gauge: IntGauge) -> Subgauge {
        Subgauge { gauge, value: 0 }
    }

    pub fn set(&mut self, new_value: i64) {
        let difference = new_value.wrapping_sub(self.value);
        self.gauge.add(difference);
        self.value = new_value;
    }
}
impl Drop for Subgauge {
    fn drop(&mut self) {
        self.gauge.sub(self.value);
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
