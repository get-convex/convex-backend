use std::{
    collections::BTreeSet,
    mem,
    time::{
        Duration,
        Instant,
    },
};

use prometheus::{
    VMHistogram,
    VMHistogramVec,
};

use crate::{
    get_desc,
    labels::StaticMetricLabel,
    log_distribution,
    log_distribution_with_labels,
};

pub trait TimerHistogram: Sized + 'static {
    fn finish(timer: &mut Timer<Self>);
}

pub struct Timer<T: TimerHistogram> {
    start: Instant,
    histogram: &'static T,
    labels: BTreeSet<StaticMetricLabel>,
}

impl<T: TimerHistogram> Drop for Timer<T> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            return;
        }
        T::finish(self);
    }
}

impl Timer<VMHistogramVec> {
    pub fn new_with_labels(histogram: &'static VMHistogramVec) -> Self {
        Self {
            start: Instant::now(),
            histogram,
            labels: BTreeSet::new(),
        }
    }

    pub fn add_label(&mut self, label: StaticMetricLabel) {
        self.labels.insert(label);
    }

    pub fn remove_label(&mut self, label: StaticMetricLabel) {
        self.labels.remove(&label);
    }

    pub fn replace_label(&mut self, old_label: StaticMetricLabel, new_label: StaticMetricLabel) {
        self.labels.remove(&old_label);
        self.labels.insert(new_label);
    }
}

impl Timer<VMHistogram> {
    pub fn new(histogram: &'static VMHistogram) -> Self {
        Self {
            start: Instant::now(),
            histogram,
            labels: BTreeSet::new(),
        }
    }
}

impl<T: TimerHistogram> Timer<T> {
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

impl TimerHistogram for VMHistogram {
    fn finish(timer: &mut Timer<Self>) {
        let elapsed_duration = timer.start.elapsed();
        let elapsed = elapsed_duration.as_secs_f64();
        let desc = get_desc(timer.histogram);
        tracing::debug!("{elapsed_duration:?} for timer {desc:?}");
        log_distribution(timer.histogram, elapsed);
    }
}

impl TimerHistogram for VMHistogramVec {
    fn finish(timer: &mut Timer<Self>) {
        let elapsed_duration = timer.start.elapsed();
        let elapsed = elapsed_duration.as_secs_f64();

        let desc = get_desc(timer.histogram);
        tracing::debug!("{elapsed_duration:?} for timer {desc:?} {:?}", timer.labels);
        let labels = mem::take(&mut timer.labels);
        log_distribution_with_labels(timer.histogram, elapsed, labels.into_iter().collect());
    }
}

/// Status timer that defaults to error unless `.finish()` is explicitly called
/// upon success.
#[derive(derive_more::Deref, derive_more::DerefMut)]
pub struct StatusTimer(Timer<VMHistogramVec>);

impl StatusTimer {
    pub fn new(histogram: &'static VMHistogramVec) -> Self {
        let mut timer = Timer::new_with_labels(histogram);
        timer.add_label(StaticMetricLabel::STATUS_ERROR);
        Self(timer)
    }

    pub fn add_label(&mut self, label: StaticMetricLabel) {
        self.0.labels.insert(label);
    }

    /// Finish the timer with status success
    pub fn finish(mut self) -> Duration {
        self.0.replace_label(
            StaticMetricLabel::STATUS_ERROR,
            StaticMetricLabel::STATUS_SUCCESS,
        );
        self.0.elapsed()
    }

    /// Finish the timer with developer error
    pub fn finish_developer_error(mut self) -> Duration {
        self.0.replace_label(
            StaticMetricLabel::STATUS_ERROR,
            StaticMetricLabel::STATUS_DEVELOPER_ERROR,
        );
        self.0.elapsed()
    }

    /// Finish the timer with the given status
    /// Commonly used as
    ///
    /// .finish_with(e.metric_status_label_value())
    pub fn finish_with(mut self, status: &'static str) -> Duration {
        self.0.replace_label(
            StaticMetricLabel::STATUS_ERROR,
            StaticMetricLabel::new("status", status),
        );
        self.0.elapsed()
    }
}

/// Timer that defaults to CANCELED, but switches to
/// ERROR/SUCCESS once you call .finish()
#[derive(derive_more::Deref, derive_more::DerefMut)]
pub struct CancelableTimer(Timer<VMHistogramVec>);

impl CancelableTimer {
    pub fn new(histogram: &'static VMHistogramVec) -> Self {
        let mut timer = Timer::new_with_labels(histogram);
        timer.add_label(StaticMetricLabel::STATUS_CANCELED);
        Self(timer)
    }

    pub fn finish(mut self, is_ok: bool) -> Duration {
        self.0.replace_label(
            StaticMetricLabel::STATUS_CANCELED,
            StaticMetricLabel::status(is_ok),
        );
        self.0.elapsed()
    }

    /// Finish the timer with developer error
    pub fn finish_developer_error(mut self) -> Duration {
        self.0.replace_label(
            StaticMetricLabel::STATUS_CANCELED,
            StaticMetricLabel::STATUS_DEVELOPER_ERROR,
        );
        self.0.elapsed()
    }

    /// Finish the timer with the given status
    /// Commonly used as
    ///
    /// .finish_with(e.metric_status_label_value())
    pub fn finish_with(mut self, status: &'static str) -> Duration {
        self.0.replace_label(
            StaticMetricLabel::STATUS_CANCELED,
            StaticMetricLabel::new("status", status),
        );
        self.0.elapsed()
    }
}
