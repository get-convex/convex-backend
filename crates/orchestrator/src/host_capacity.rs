//! Read host total memory + CPU. Refreshed lazily — callers ask via
//! `read()`, the inner cache returns the last value if it's < 60s old.

use std::{
    sync::Mutex,
    time::{
        Duration,
        Instant,
    },
};

const REFRESH_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy)]
pub struct HostCapacity {
    pub total_memory_mb: u64,
    pub total_cpus: u32,
}

struct Cache {
    last_read: Option<(Instant, HostCapacity)>,
}

pub struct HostCapacityReader {
    cache: Mutex<Cache>,
}

impl HostCapacityReader {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(Cache { last_read: None }),
        }
    }

    pub fn read(&self) -> HostCapacity {
        let mut cache = self.cache.lock().expect("host-capacity cache poisoned");
        if let Some((at, cap)) = cache.last_read
            && at.elapsed() < REFRESH_INTERVAL
        {
            return cap;
        }
        let cap = read_uncached();
        cache.last_read = Some((Instant::now(), cap));
        cap
    }
}

impl Default for HostCapacityReader {
    fn default() -> Self {
        Self::new()
    }
}

fn read_uncached() -> HostCapacity {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    sys.refresh_cpu_all();
    HostCapacity {
        // sysinfo returns total_memory() in bytes since 0.30; clamp + convert.
        total_memory_mb: (sys.total_memory() / 1024 / 1024).max(1),
        total_cpus: sys.cpus().len().max(1) as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reader_returns_nonzero_values() {
        let r = HostCapacityReader::new();
        let cap = r.read();
        assert!(cap.total_memory_mb > 0);
        assert!(cap.total_cpus > 0);
    }

    #[test]
    fn second_read_is_cached() {
        let r = HostCapacityReader::new();
        let first = r.read();
        let second = r.read();
        assert_eq!(first.total_memory_mb, second.total_memory_mb);
        assert_eq!(first.total_cpus, second.total_cpus);
    }
}
