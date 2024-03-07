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
    log_distribution,
    log_distribution_with_tags,
    metric_tag_const_value,
    tags::MetricTag,
};

pub struct Timer<T: 'static> {
    start: Instant,
    histogram: &'static T,
    tags: BTreeSet<MetricTag>,
}

trait DropInner {
    fn drop_inner(&mut self);
}

impl<T: 'static> DropInner for Timer<T> {
    default fn drop_inner(&mut self) {
        panic!("Default Drop implementation for Timer should not be callable")
    }
}

impl<T: 'static> Drop for Timer<T> {
    fn drop(&mut self) {
        self.drop_inner();
    }
}

impl Timer<VMHistogramVec> {
    pub fn new_tagged(histogram: &'static VMHistogramVec) -> Self {
        Self {
            start: Instant::now(),
            histogram,
            tags: BTreeSet::new(),
        }
    }

    pub fn add_tag(&mut self, tag: MetricTag) {
        self.tags.insert(tag);
    }

    pub fn remove_tag(&mut self, tag: MetricTag) {
        self.tags.remove(&tag);
    }

    pub fn replace_tag(&mut self, old_tag: MetricTag, new_tag: MetricTag) {
        self.tags.remove(&old_tag);
        self.tags.insert(new_tag);
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

impl Timer<VMHistogram> {
    pub fn new(histogram: &'static VMHistogram) -> Self {
        Self {
            start: Instant::now(),
            histogram,
            tags: BTreeSet::new(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

impl DropInner for Timer<VMHistogram> {
    fn drop_inner(&mut self) {
        if std::thread::panicking() {
            return;
        }
        let elapsed_duration = self.start.elapsed();
        let elapsed = elapsed_duration.as_secs_f64();
        let desc = get_desc(self.histogram);
        tracing::debug!("{elapsed_duration:?} for timer {desc:?}");
        log_distribution(self.histogram, elapsed);
    }
}

impl DropInner for Timer<VMHistogramVec> {
    fn drop_inner(&mut self) {
        if std::thread::panicking() {
            return;
        }
        let elapsed_duration = self.start.elapsed();
        let elapsed = elapsed_duration.as_secs_f64();

        let desc = get_desc(self.histogram);
        tracing::debug!("{elapsed_duration:?} for timer {desc:?} {:?}", self.tags);
        let tags = mem::take(&mut self.tags);
        log_distribution_with_tags(self.histogram, elapsed, tags.into_iter().collect());
    }
}

/// Status timer that defaults to error unless `.finish()` is explicitly called
/// upon success.
#[derive(derive_more::Deref, derive_more::DerefMut)]
pub struct StatusTimer(Timer<VMHistogramVec>);

impl StatusTimer {
    pub fn new(histogram: &'static VMHistogramVec) -> Self {
        let mut timer = Timer::new_tagged(histogram);
        timer.add_tag(MetricTag::STATUS_ERROR);
        Self(timer)
    }

    pub fn add_tag(&mut self, tag: MetricTag) {
        self.0.tags.insert(tag);
    }

    /// Finish the timer with status success
    pub fn finish(mut self) -> Duration {
        self.0
            .replace_tag(MetricTag::STATUS_ERROR, MetricTag::STATUS_SUCCESS);
        self.0.elapsed()
    }

    /// Finish the timer with developer error
    pub fn finish_developer_error(mut self) -> Duration {
        self.0
            .replace_tag(MetricTag::STATUS_ERROR, MetricTag::STATUS_DEVELOPER_ERROR);
        self.0.elapsed()
    }

    /// Finish the timer with the given status
    /// Commonly used as
    ///
    /// .finish_with(e.metric_status_tag_value())
    pub fn finish_with(mut self, status: &'static str) -> Duration {
        self.0.replace_tag(
            MetricTag::STATUS_ERROR,
            metric_tag_const_value("status", status),
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
        let mut timer = Timer::new_tagged(histogram);
        timer.add_tag(MetricTag::STATUS_CANCELED);
        Self(timer)
    }

    pub fn finish(mut self, is_ok: bool) -> Duration {
        self.0
            .replace_tag(MetricTag::STATUS_CANCELED, MetricTag::status(is_ok));
        self.0.elapsed()
    }

    /// Finish the timer with developer error
    pub fn finish_developer_error(mut self) -> Duration {
        self.0.replace_tag(
            MetricTag::STATUS_CANCELED,
            MetricTag::STATUS_DEVELOPER_ERROR,
        );
        self.0.elapsed()
    }

    /// Finish the timer with the given status
    /// Commonly used as
    ///
    /// .finish_with(e.metric_status_tag_value())
    pub fn finish_with(mut self, status: &'static str) -> Duration {
        self.0.replace_tag(
            MetricTag::STATUS_CANCELED,
            metric_tag_const_value("status", status),
        );
        self.0.elapsed()
    }
}
