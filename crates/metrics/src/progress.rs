use std::mem;

use prometheus::IntCounterVec;

use crate::{
    get_desc,
    log_counter_with_tags,
    MetricTag,
};

/// Logs a counter of the number of results left unprocessed if the counter is
/// dropped before calling `complete`. Create a ProgressCounter at the beginning
/// of a stream and mark it `complete` at the end of the stream when the stream
/// might be leaving resources behind when it is dropped or errors.
/// The logged counter is (estimated_max_total - processed_count) at drop time.
/// Since estimated_max_total may be an overestimate, we are logging an *upper
/// bound*.
///
/// For example, when consuming a stream of results from MySQL, if we error
/// early, MySQL still might send many results across the connection, and they
/// will need to be drained at some point.
pub struct ProgressCounter {
    estimated_max_total: usize,
    processed_count: usize,
    complete: bool,
    unfinished_progress_counter: &'static IntCounterVec,
    tags: Vec<MetricTag>,
}

impl ProgressCounter {
    pub fn new(
        unfinished_progress_counter: &'static IntCounterVec,
        estimated_max_total: usize,
        tags: Vec<MetricTag>,
    ) -> Self {
        Self {
            estimated_max_total,
            processed_count: 0,
            complete: false,
            unfinished_progress_counter,
            tags,
        }
    }

    pub fn add_processed(&mut self, newly_processed: usize) {
        self.processed_count += newly_processed;
    }

    pub fn complete(&mut self) {
        self.complete = true;
    }
}

impl Drop for ProgressCounter {
    fn drop(&mut self) {
        if std::thread::panicking() {
            return;
        }
        if self.complete || self.processed_count >= self.estimated_max_total {
            return;
        }
        let unfinished_progress = self.estimated_max_total - self.processed_count;
        let desc = get_desc(self.unfinished_progress_counter);
        tracing::debug!(
            "unfinished progress {unfinished_progress} for {desc:?} {:?}",
            self.tags
        );
        let tags = mem::take(&mut self.tags);
        log_counter_with_tags(
            self.unfinished_progress_counter,
            unfinished_progress as u64,
            tags,
        );
    }
}
