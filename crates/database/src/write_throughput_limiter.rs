use std::collections::VecDeque;

use common::{
    knobs::{
        MAX_BYTES_WRITTEN_PER_SECOND,
        PROPOSED_MAX_BYTES_WRITTEN_PER_SECOND,
        WRITE_THROUGHPUT_WINDOW,
    },
    types::Timestamp,
};

use crate::metrics::{
    log_write_throughput,
    log_write_throughput_limit_exceeded,
    log_write_throughput_limit_would_be_exceeded,
};

/// Tracks write throughput and enforces rate limits on database writes.
///
/// This limiter maintains a sliding window of write events and their byte
/// sizes, allowing efficient O(1) throughput checks by keeping a running total.
pub struct WriteThroughputLimiter {
    /// Bytes written in transactions that committed at each timestamp.
    bytes_written: VecDeque<(Timestamp, u64)>,
    /// Running total of bytes in bytes_written for O(1) throughput checks.
    total_bytes_written_in_window: u64,
    /// Maximum number of bytes that can be written in the given window.
    max_bytes_written_in_window: u64,
    /// Proposed maximum number of bytes that can be written in the given window
    proposed_max_bytes_written_in_window: u64,
}

impl WriteThroughputLimiter {
    pub fn new() -> Self {
        Self {
            bytes_written: VecDeque::new(),
            total_bytes_written_in_window: 0,
            max_bytes_written_in_window: *MAX_BYTES_WRITTEN_PER_SECOND
                * WRITE_THROUGHPUT_WINDOW.as_millis() as u64
                / 1000,
            proposed_max_bytes_written_in_window: *PROPOSED_MAX_BYTES_WRITTEN_PER_SECOND
                * WRITE_THROUGHPUT_WINDOW.as_millis() as u64
                / 1000,
        }
    }

    pub fn record_write(&mut self, ts: Timestamp, write_bytes: u64) {
        // Clean up old write events outside the write throughput window
        while let Some((event_ts, _)) = self.bytes_written.front() {
            if (ts - *event_ts) > *WRITE_THROUGHPUT_WINDOW {
                if let Some((_, old_write_bytes)) = self.bytes_written.pop_front() {
                    self.total_bytes_written_in_window = self
                        .total_bytes_written_in_window
                        .saturating_sub(old_write_bytes);
                }
            } else {
                break;
            }
        }

        // Track new write event for throughput limiting
        self.bytes_written.push_back((ts, write_bytes));
        self.total_bytes_written_in_window += write_bytes;
    }

    pub fn check_limit(&self, current_ts: Timestamp) -> bool {
        log_write_throughput(self.total_bytes_written_in_window);
        if self.total_bytes_written_in_window > self.max_bytes_written_in_window {
            // N.B. Check the actual bytes written in the window relative to
            // the current ts passed in, because old_entries are only evicted
            // on writes.
            let actual_bytes_written_in_window: u64 = self
                .bytes_written
                .iter()
                .filter(|(event_ts, _)| {
                    current_ts < *event_ts || (current_ts - *event_ts) <= *WRITE_THROUGHPUT_WINDOW
                })
                .map(|(_, bytes)| bytes)
                .sum();
            if actual_bytes_written_in_window > self.max_bytes_written_in_window {
                log_write_throughput_limit_exceeded();
                false
            } else {
                true
            }
        } else {
            if self.total_bytes_written_in_window > self.proposed_max_bytes_written_in_window {
                log_write_throughput_limit_would_be_exceeded();
            }
            true
        }
    }
}
