use std::{
    fmt,
    time::Duration,
};

use humansize::{
    FormatSize,
    BINARY,
};
use parking_lot::Mutex;
use serde::Deserialize;
use textplots::{
    Chart,
    Plot,
    Shape,
};

pub fn print_histogram(mut timings: Vec<Duration>) {
    if timings.is_empty() {
        return;
    }
    timings.sort_unstable();

    let mean = (timings.iter().sum::<Duration>()) / timings.len() as u32;
    let percentile = |p: f64| timings[(timings.len() as f64 * (p / 100.)) as usize];
    println!("Total: {}", timings.len());
    println!("Min  : {:.2?}", timings[0]);
    println!("Mean : {mean:.2?}");
    println!("p10  : {:.2?}", percentile(10.));
    println!("p50  : {:.2?}", percentile(50.));
    println!("p75  : {:.2?}", percentile(75.));
    println!("p95  : {:.2?}", percentile(95.));
    println!("p99  : {:.2?}", percentile(99.));
    println!("p99.5: {:.2?}", percentile(99.5));
    println!("Max  : {:.2?}", timings.last().unwrap());

    // Convert to milliseconds for the histogram.
    let timings_ms = timings
        .iter()
        .map(|t| (t.as_secs_f64() * 1000.) as f32)
        .collect::<Vec<_>>();
    let num_buckets = 100;
    let data_start = timings_ms[0];
    let data_end = *timings_ms.last().unwrap() + 1.;
    let data_width = data_end - data_start;
    let bucket_width = data_width / num_buckets as f32;

    let mut buckets = vec![0; num_buckets];
    for &sample in &timings_ms {
        buckets[((sample - data_start) / bucket_width) as usize] += 1;
    }
    let mut data = vec![(0., 0.), (data_start - 1., 0.)];
    data.extend(buckets.into_iter().enumerate().map(|(i, count)| {
        let bucket_midpoint = data_start + (i as f32 + 0.5) * bucket_width;
        let proportion = (count as f32) / (timings_ms.len() as f32);
        (bucket_midpoint, proportion)
    }));
    data.push((data_end + 1., 0.));
    data.push((data_end * 1.25, 0.));

    println!();
    Chart::new(180, 60, 0., data_end * 1.25)
        .lineplot(&Shape::Lines(&data))
        .nice();
}

/// Docstrings taken from http://jemalloc.net/jemalloc.3.html
#[derive(Deserialize)]
pub struct JemallocStats {
    /// Total number of bytes allocated by the application.
    pub allocated: usize,
    /// Total number of bytes in active pages allocated by the application. This
    /// is a multiple of the page size, and greater than or equal to
    /// stats.allocated. This does not include stats.arenas.<i>.pdirty,
    /// stats.arenas.<i>.pmuzzy, nor pages entirely devoted to allocator
    /// metadata.
    pub active: usize,
    /// Total number of bytes dedicated to metadata, which comprise base
    /// allocations used for bootstrap-sensitive allocator metadata structures
    /// (see stats.arenas.<i>.base) and internal allocations (see
    /// stats.arenas.<i>.internal). Transparent huge page (enabled with
    /// opt.metadata_thp) usage is not considered.
    pub metadata: usize,
    /// Number of transparent huge pages (THP) used for metadata.
    pub metadata_thp: usize,
    /// Maximum number of bytes in physically resident data pages mapped by the
    /// allocator, comprising all pages dedicated to allocator metadata, pages
    /// backing active allocations, and unused dirty pages. This is a maximum
    /// rather than precise because pages may not actually be physically
    /// resident if they correspond to demand-zeroed virtual memory that has not
    /// yet been touched. This is a multiple of the page size, and is larger
    /// than stats.active.
    pub resident: usize,
    /// Total number of bytes in active extents mapped by the allocator. This is
    /// larger than stats.active. This does not include inactive extents, even
    /// those that contain unused dirty pages, which means that there is no
    /// strict ordering between this and stats.resident.
    pub mapped: usize,
    /// Total number of bytes in virtual memory mappings that were retained
    /// rather than being returned to the operating system via e.g. munmap(2) or
    /// similar. Retained virtual memory is typically untouched, decommitted, or
    /// purged, so it has no strongly associated physical memory (see extent
    /// hooks for details). Retained memory is excluded from mapped memory
    /// statistics, e.g. stats.mapped.
    pub retained: usize,
    /// Number of times that the realloc() was called with a non-NULL pointer
    /// argument and a 0 size argument. This is a fundamentally unsafe pattern
    /// in portable programs; see opt.zero_realloc for details.
    pub zero_reallocs: usize,
    pub background_thread: JemallocBackgroundThread,
}

// `humansize`'s formatter doesn't implement `Debug` and requires specifying the
// format each time, where we always want BINARY.
pub struct SizeFormatter(pub usize);

impl fmt::Debug for SizeFormatter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.format_size(BINARY))
    }
}

impl fmt::Debug for JemallocStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JemallocStats")
            .field("allocated", &SizeFormatter(self.allocated))
            .field("active", &SizeFormatter(self.active))
            .field("metadata", &SizeFormatter(self.metadata))
            .field("metadata_thp", &self.metadata_thp)
            .field("resident", &SizeFormatter(self.resident))
            .field("mapped", &SizeFormatter(self.mapped))
            .field("retained", &SizeFormatter(self.retained))
            .field("zero_reallocs", &self.zero_reallocs)
            .field("background_thread", &self.background_thread)
            .finish()
    }
}

#[derive(Deserialize, Debug)]
pub struct JemallocBackgroundThread {
    /// Number of background threads running currently.
    pub num_threads: usize,
    /// Total number of runs from all background threads.
    pub num_runs: usize,
    /// Average run interval in nanoseconds of background threads.
    pub run_interval: usize,
}

pub static JEMALLOC_STATS_REPORTER: Mutex<Option<fn() -> anyhow::Result<JemallocStats>>> =
    Mutex::new(None);
