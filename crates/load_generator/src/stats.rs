use std::{
    cell::RefCell,
    collections::BTreeMap,
    fs::{
        self,
        File,
    },
    io::{
        BufRead,
        BufReader,
    },
    path::PathBuf,
    rc::Rc,
    time::Duration,
};

use performance_stats::performance::print_histogram;
use serde_json::Value;

use crate::event_receiver::{
    Event,
    EventMetadata,
    Metric,
    MetricType,
};

/// Corresponds to the number of logs emitted in log.ts
const LOG_LINES_PER_LOG_ACTION: usize = 256;
/// Corresponds to the length of a log line emitted in log.ts
const LOG_LINE_LENGTH: usize = 512;

#[derive(Clone)]
pub struct Stats {
    duration: Duration,
    inner: Rc<RefCell<StatsInner>>,
    local_log_sink: Option<PathBuf>,
}

struct StatsInner {
    counts: BTreeMap<EventMetadata, u64>,
    latencies: BTreeMap<EventMetadata, Vec<f64>>,
    errors: BTreeMap<EventMetadata, u64>,
}

impl Stats {
    pub fn new(duration: Duration, local_log_sink: Option<PathBuf>) -> Self {
        Self {
            duration,
            inner: Rc::new(RefCell::new(StatsInner {
                counts: BTreeMap::new(),
                latencies: BTreeMap::new(),
                errors: BTreeMap::new(),
            })),
            local_log_sink,
        }
    }

    fn record_latency(&mut self, value: f64, event_metadata: EventMetadata) {
        let mut inner = self.inner.borrow_mut();
        match inner.latencies.get_mut(&event_metadata) {
            None => {
                inner.latencies.insert(event_metadata, vec![value]);
            },
            Some(q) => {
                q.push(value);
            },
        }
    }

    fn record_count(&mut self, value: u64, event_metadata: EventMetadata) {
        let mut inner = self.inner.borrow_mut();
        let new_count = inner
            .counts
            .get(&event_metadata)
            .map(|count| count + value)
            .unwrap_or_else(|| value);
        inner.counts.insert(event_metadata, new_count);
    }

    fn record_error(&mut self, event_metadata: EventMetadata) {
        let mut inner = self.inner.borrow_mut();
        let new_count = inner
            .errors
            .get(&event_metadata)
            .map(|count| count + 1)
            .unwrap_or_else(|| 1);
        inner.errors.insert(event_metadata, new_count);
    }

    pub fn process_event(&mut self, event: Event) {
        let Event { metadata, metric } = event;

        match metric {
            Metric::Error { msg } => {
                tracing::error!(
                    "ScenarioRunner reported error: {msg} with name {} from scenario {}",
                    metadata.name,
                    metadata.scenario,
                );
                self.record_error(metadata)
            },
            Metric::Metric {
                r#type: MetricType::Latency,
                value,
            } => self.record_latency(value, metadata),
            Metric::Metric {
                r#type: MetricType::Count,
                value,
            } => self.record_count(value as u64, metadata),
        }
    }

    fn histogram(&self, arr: &Vec<f64>) {
        let timings: Vec<Duration> = arr.iter().map(|x| Duration::from_secs_f64(*x)).collect();
        print_histogram(timings);
    }

    /// Some errors are expected, but if there are too many errors recorded in
    /// [`Stats`], fail.
    pub fn fail_if_too_many_errors(&self) -> anyhow::Result<()> {
        let num_errors = self.inner.borrow().errors.values().sum::<u64>();
        let duration_in_secs = self.duration.as_secs();
        // If there are more than an error per second, fail.
        anyhow::ensure!(
            num_errors < duration_in_secs,
            "Too many errors recorded in stats: {num_errors} errors over {duration_in_secs} \
             seconds",
        );
        Ok(())
    }

    pub fn report(&self) {
        let inner = self.inner.borrow();

        println!("Latencies");
        println!("======");
        for (
            EventMetadata {
                name,
                scenario,
                path,
            },
            q,
        ) in inner.latencies.iter()
        {
            println!("Stats for {name} in scenario {scenario} with path {path:?}:");
            println!("QPS: {:.2}", (q.len() as f64) / self.duration.as_secs_f64());
            self.histogram(q);
        }
        println!();
        println!();

        println!("Counts");
        println!("======");
        for (
            EventMetadata {
                name,
                scenario,
                path,
            },
            count,
        ) in inner.counts.iter()
        {
            println!("Rate for {name} in scenario {scenario} with path {path:?}: {count:?}");
        }
        println!();
        println!();

        println!("Errors");
        println!("======");
        for (
            EventMetadata {
                name,
                scenario,
                path,
            },
            count,
        ) in inner.errors.iter()
        {
            println!("Errors for {name} in scenario {scenario} with path {path:?}: {count:?}");
        }
        println!();
        println!();

        let log_event_metadata = EventMetadata {
            name: "action".to_string(),
            scenario: "RunFunction".to_string(),
            path: Some("log".to_string()),
        };
        if let Some(log_action_latencies) = inner.latencies.get(&log_event_metadata)
            && let Some(local_log_sink) = self.local_log_sink.as_ref()
        {
            let log_action_count = log_action_latencies.len();
            let log_sink_metadata = fs::metadata(local_log_sink).unwrap_or_else(|_| {
                panic!(
                    "Failed to get metadata from local log sink at {}",
                    local_log_sink.to_string_lossy()
                )
            });
            // Count log lines in `_console` and `_execution_record` topic
            let expected_log_lines = log_action_count * LOG_LINES_PER_LOG_ACTION + log_action_count;
            let file = File::open(local_log_sink).unwrap();
            let buf_reader = BufReader::new(file);
            let mut log_lines = 0;

            for line in buf_reader.lines() {
                let line = line.unwrap();
                let json: Value = serde_json::from_str(&line).unwrap();
                let object = json.as_object().unwrap();
                let topic = object.get("topic").unwrap();
                if topic != "function_execution" && topic != "console" {
                    continue;
                }
                let path = object
                    .get("function")
                    .unwrap()
                    .get("path")
                    .unwrap()
                    .as_str()
                    .unwrap();
                if !path.contains("log") {
                    continue;
                }
                log_lines += 1;
            }
            println!("Logs");
            println!("======");
            if log_sink_metadata.len() < log_lines as u64 * LOG_LINE_LENGTH as u64 {
                println!("WARNING: Log sink size is less than expected!");
            }
            println!("Expected: {} lines", expected_log_lines);
            println!("Received: {} lines", log_lines);
            println!(
                "Drop rate: {:.2}%",
                100.0 * (1.0 - (log_lines as f64) / (expected_log_lines as f64))
            );
        }
        println!();
        println!("Stats report complete!");
    }
}
