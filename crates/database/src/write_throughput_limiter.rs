use std::collections::VecDeque;

use common::{
    knobs::{
        MAX_BYTES_WRITTEN_PER_SECOND,
        WRITE_THROUGHPUT_WINDOW,
    },
    types::Timestamp,
};

use crate::metrics::log_write_throughput_limit_exceeded;

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
}

impl WriteThroughputLimiter {
    pub fn new() -> Self {
        Self {
            bytes_written: VecDeque::new(),
            total_bytes_written_in_window: 0,
            max_bytes_written_in_window: *MAX_BYTES_WRITTEN_PER_SECOND
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

    pub fn check_limit(&self) -> bool {
        if self.total_bytes_written_in_window > self.max_bytes_written_in_window {
            log_write_throughput_limit_exceeded();
            false
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use common::{
        knobs::WRITE_THROUGHPUT_WINDOW,
        types::Timestamp,
    };

    use super::WriteThroughputLimiter;

    #[test]
    fn test_write_throughput_limiter_allows_writes_under_limit() {
        let mut limiter = WriteThroughputLimiter::new();
        let ts = Timestamp::MIN;

        // Write just under the limit
        let bytes_under_limit = limiter.max_bytes_written_in_window - 1;
        limiter.record_write(ts, bytes_under_limit);
        assert!(limiter.check_limit());
    }

    #[test]
    fn test_write_throughput_limiter_blocks_writes_over_limit() {
        let mut limiter = WriteThroughputLimiter::new();
        let ts = Timestamp::MIN;

        // Write over the limit
        let bytes_over_limit = limiter.max_bytes_written_in_window + 1;
        limiter.record_write(ts, bytes_over_limit);
        assert!(!limiter.check_limit());
    }

    #[test]
    fn test_write_throughput_limiter_cleans_up_old_writes() {
        let mut limiter = WriteThroughputLimiter::new();
        let ts1 = Timestamp::MIN;

        // Write over the limit
        limiter.record_write(ts1, limiter.max_bytes_written_in_window + 1);

        // Should fail
        assert!(!limiter.check_limit());

        // Move forward past the window
        let ts2 = ts1
            .add(*WRITE_THROUGHPUT_WINDOW)
            .unwrap()
            .add(Duration::from_millis(1))
            .unwrap();
        limiter.record_write(ts2, 100);

        // Should succeed now since the old write is outside the window
        assert!(limiter.check_limit());
        assert_eq!(limiter.total_bytes_written_in_window, 100);
    }

    #[test]
    fn test_write_throughput_limiter_accumulates_writes_in_window() {
        let mut limiter = WriteThroughputLimiter::new();
        let ts1 = Timestamp::MIN;

        // First write
        let half_limit = limiter.max_bytes_written_in_window / 2;
        limiter.record_write(ts1, half_limit);
        assert!(limiter.check_limit());

        // Second write within window
        let ts2 = ts1.add(Duration::from_millis(100)).unwrap();
        limiter.record_write(ts2, half_limit + 1);

        // Should fail since we're over the limit
        assert!(!limiter.check_limit());
    }
}
