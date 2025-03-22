use std::{
    collections::BTreeMap,
    sync::LazyLock,
};

use anyhow::Context;
use maplit::btreemap;
use metrics::{
    get_desc,
    log_counter_with_labels,
    log_distribution_with_labels,
    log_gauge_with_labels,
    register_convex_counter_owned,
    register_convex_gauge,
    register_convex_histogram_owned,
    MetricLabel,
};
use prometheus::{
    IntCounterVec,
    VMHistogramVec,
};

use crate::event_receiver::{
    Event,
    EventMetadata,
    Metric,
    MetricType,
};

register_convex_gauge!(
    SCENARIO_TARGET_QPS_TOTAL,
    "The target QPS for a scenario",
    &["scenario", "path"],
);
pub fn log_target_qps(scenario: &str, qps: f64, path: Option<String>) {
    log_gauge_with_labels(
        &SCENARIO_TARGET_QPS_TOTAL,
        qps,
        vec![
            MetricLabel::new("scenario", scenario),
            MetricLabel::new("path", path.as_deref().unwrap_or("none")),
        ],
    );
}

// Scenario -> Name -> Metric
static ERROR_METRICS: LazyLock<BTreeMap<&str, BTreeMap<&str, IntCounterVec>>> =
    LazyLock::new(|| {
        btreemap! {
            "RunFunction" => btreemap! {
                "query" => {
                    register_convex_counter_owned!(
                        QUERY_ERROR_TOTAL,
                        "Query errors on the RunFunction scenario",
                        &["backend_version", "load_description"],
                    )
                },
                "mutation" => {
                    register_convex_counter_owned!(
                        MUTATION_ERROR_TOTAL,
                        "Mutation errors on the RunFunction scenario",
                        &["backend_version", "load_description"],
                    )
                },
                "action" => {
                    register_convex_counter_owned!(
                        ACTION_ERROR_TOTAL,
                        "Action errors on the RunFunction scenario",
                        &["backend_version", "load_description"],
                    )
                }
            },
            "RunHttpAction" => btreemap! {
                "http_action" => {
                    register_convex_counter_owned!(
                        HTTP_ACTION_ERROR_TOTAL,
                        "HTTP action errors on the RunHttpAction scenario",
                        &["backend_version", "load_description"],
                    )
                },
            },
            "ObserveInsert" => btreemap!{
                "mutation" => {
                    register_convex_counter_owned!(
                        OBSERVE_INSERT_ERROR_TOTAL,
                        "Errors on the insert mutation scenario",
                        &["backend_version", "load_description"],
                    )
                }
            },
            "SnapshotExport" => btreemap!{
                "request_export_failed" => {
                    register_convex_counter_owned!(
                        SNAPSHOT_EXPORT_REQUEST_FAILED_TOTAL,
                        "Errors on requesting a snapshot export",
                        &["backend_version", "load_description"],
                    )
                },
                "get_export_failed" => {
                    register_convex_counter_owned!(
                        SNAPSHOT_EXPORT_GET_EXPORT_FAILED_TOTAL,
                        "Errors on requesting a snapshot export",
                        &["backend_version", "load_description"],
                    )
                },
                "snapshot_export_failed" => {
                    register_convex_counter_owned!(
                        SNAPSHOT_EXPORT_FAILED_TOTAL,
                        "Errors on snapshot export",
                        &["backend_version", "load_description"],
                    )
                },
                "snapshot_failure" => {
                    register_convex_counter_owned!(
                        SNAPSHOT_FAILED_TOTAL,
                        "Unexpected errors on snapshot scenarios",
                        &["backend_version", "load_description"],
                    )
                }
            },
            "CloudBackup" => btreemap!{
                "request_backup_failed" => {
                    register_convex_counter_owned!(
                        CLOUD_BACKUP_REQUEST_FAILED_TOTAL,
                        "Errors on requesting a cloud backup",
                        &["backend_version", "load_description"],
                    )
                },
                "get_backup_failed" => {
                    register_convex_counter_owned!(
                        CLOUD_BACKUP_GET_FAILED_TOTAL,
                        "Errors on requesting a cloud backup",
                        &["backend_version", "load_description"],
                    )
                },
                "backup_failed" => {
                    register_convex_counter_owned!(
                        CLOUD_BACKUP_FAILED_TOTAL,
                        "Errors on completing a cloud backup",
                        &["backend_version", "load_description"],
                    )
                },
                "backup_failure" => {
                    register_convex_counter_owned!(
                        BACKUP_FAILED_TOTAL,
                        "Unexpected errors on backup scenarios",
                        &["backend_version", "load_description"],
                    )
                }
            },
            "Search" => btreemap!{
                "search_document_mismatch" => {
                    register_convex_counter_owned!(
                        SEARCH_DOCUMENT_MISMATCH_TOTAL,
                        "Errors on document mismatch in Search",
                        &["backend_version", "load_description"],
                    )
                },
                "search" => {
                    register_convex_counter_owned!(
                        SEARCH_ERROR_TOTAL,
                        "Errors in Search",
                        &["backend_version", "load_description"],
                    )
                }
            },
            "VectorSearch" => btreemap!{
                "vector_search_document_mismatch" => {
                    register_convex_counter_owned!(
                        VECTOR_SEARCH_DOCUMENT_MISMATCH_TOTAL,
                        "Errors on document mismatch in VectorSearch",
                        &["backend_version", "load_description"],
                    )
                },
                "vector_search" => {
                    register_convex_counter_owned!(
                        VECTOR_SEARCH_ERROR_TOTAL,
                        "Errors in VectorSearch",
                        &["backend_version", "load_description"],
                    )
                }
            },
        }
    });
// Scenario -> Name -> Metric
static LATENCY_METRICS: LazyLock<BTreeMap<&str, BTreeMap<&str, VMHistogramVec>>> =
    LazyLock::new(|| {
        btreemap! {
            "RunFunction" => btreemap!{
                "query" => {
                    register_convex_histogram_owned!(
                        QUERY_LATENCY_SECONDS,
                        "Latency for the query function",
                        &["backend_version", "load_description", "path"],
                    )
                },
                "mutation" => {
                    register_convex_histogram_owned!(
                        MUTATION_LATENCY_SECONDS,
                        "Latency for the mutation function",
                        &["backend_version", "load_description", "path"],
                    )
                },
                "action" => {
                    register_convex_histogram_owned!(
                        ACTION_LATENCY_SECONDS,
                        "Latency for the action function",
                        &["backend_version", "load_description", "path"],
                    )
                }
            },
            "RunHttpAction" => btreemap!{
                "http_action" => {
                    register_convex_histogram_owned!(
                        HTTP_ACTION_LATENCY_SECONDS,
                        "Latency for an HTTP action",
                        &["backend_version", "load_description", "path"],
                    )
                },
            },
            "ObserveInsert" => btreemap!{
                "mutation_completed" => {
                    register_convex_histogram_owned!(
                        OBSERVE_INSERT_COMPLETED_LATENCY_SECONDS,
                        "Latency on mutation completion for ObserveInsert",
                        &["backend_version", "load_description"],
                    )
                },
                "mutation_observed" => {
                    register_convex_histogram_owned!(
                        OBSERVE_INSERT_OBSERVED_LATENCY_SECONDS,
                        "Latency on observing mutation for ObserveInsert",
                        &["backend_version", "load_description"],
                    )
                }
            },
            "ObserveInsertWithSearch" => btreemap!{
                "mutation_completed" => {
                    register_convex_histogram_owned!(
                        OBSERVE_INSERT_WITH_SEARCH_COMPLETED_LATENCY_SECONDS,
                        "Latency on mutation completion for ObserveInsertWithSearch",
                        &["backend_version", "load_description"],
                    )
                },
                "mutation_observed" => {
                    register_convex_histogram_owned!(
                        OBSERVE_INSERT_WITH_SEARCH_OBSERVED_LATENCY_SECONDS,
                        "Latency on observing mutation for ObserveInsertWithSearch",
                        &["backend_version", "load_description"],
                    )
                }
            },
            "Search" => btreemap!{
                "search" => {
                    register_convex_histogram_owned!(
                        SEARCH_LATENCY_SECONDS,
                        "Latency on the search scenario",
                        &["backend_version", "load_description"],
                    )
                }
            },
            "VectorSearch" => btreemap!{
                "vector_search" => {
                    register_convex_histogram_owned!(
                        VECTOR_SEARCH_LATENCY_SECONDS,
                        "Latency on the vector search scenario",
                        &["backend_version", "load_description"],
                    )
                }
            },
            "CloudBackup" => btreemap!{
                "backup" => {
                    register_convex_histogram_owned!(
                        BACKUP_LATENCY_SECONDS,
                        "Latency on the backup scenario",
                        &["backend_version", "load_description"],
                    )
                },
            },
        }
    });
// Scenario -> Name -> Metric
static COUNT_METRICS: LazyLock<BTreeMap<&str, BTreeMap<&str, IntCounterVec>>> =
    LazyLock::new(|| {
        btreemap! {
            "RunFunction" => btreemap!{
                "query_timeout" => {
                    register_convex_counter_owned!(
                        QUERY_TIMEOUT_TOTAL,
                        "Number of query timeouts on RunFunction scenario",
                        &["backend_version", "load_description", "path"],
                    )
                },
                "mutation_timeout" => {
                    register_convex_counter_owned!(
                        MUTATION_TIMEOUT_TOTAL,
                        "Number of mutation timeouts on RunFunction scenario",
                        &["backend_version", "load_description", "path"],
                    )
                },
                "action_timeout" => {
                    register_convex_counter_owned!(
                        ACTION_TIMEOUT_TOTAL,
                        "Number of action timeouts on RunFunction scenario",
                        &["backend_version", "load_description", "path"],
                    )
                }
            },
            "RunHttpAction" => btreemap! {
                "http_action_timeout" => {
                    register_convex_counter_owned!(
                        HTTP_ACTION_TIMEOUT_TOTAL,
                        "Number of HTTP action timeouts on RunHttpAction scenario",
                        &["backend_version", "load_description", "path"],
                    )
                }
            },
            "ObserveInsert" => btreemap!{
                "mutation_send_timeout" => {
                    register_convex_counter_owned!(
                        OBSERVE_INSERT_SEND_TIMEOUT_TOTAL,
                        "Count of send timeouts for mutation in ObserveInsert",
                        &["backend_version", "load_description"],
                    )
                },
                "mutation_observed_timeout" => {
                    register_convex_counter_owned!(
                        OBSERVE_INSERT_OBSERVED_TIMEOUT_TOTAL,
                        "Count of observed timeouts for mutation in ObserveInsert",
                        &["backend_version", "load_description"],
                    )
                }
            },
            "ObserveInsertWithSearch" => btreemap!{
                "mutation_send_timeout" => {
                    register_convex_counter_owned!(
                        OBSERVE_INSERT_WITH_SEARCH_SEND_TIMEOUT_TOTAL,
                        "Count of send timeouts for mutation in ObserveInsertWithSearch",
                        &["backend_version", "load_description"],
                    )
                },
                "mutation_observed_timeout" => {
                    register_convex_counter_owned!(
                        OBSERVE_INSERT_WITH_SEARCH_OBSERVED_TIMEOUT_TOTAL,
                        "Count of observed timeouts for mutation in ObserveInsertWithSearch",
                        &["backend_version", "load_description"],
                    )
                }
            },
            "SnapshotExport" => btreemap!{
                "export_timeout" => {
                    register_convex_counter_owned!(
                        SNAPSHOT_EXPORT_TIMEOUT_TOTAL,
                        "Count of export timeouts in SnapshotExport",
                        &["backend_version", "load_description"],
                    )
                },
                "request_export_succeeded" => {
                    register_convex_counter_owned!(
                        SNAPSHOT_REQUEST_EXPORT_SUCCEEDED_TOTAL,
                        "Count of successful export requests in SnapshotExport",
                        &["backend_version", "load_description"],
                    )
                },
                "export_completed" => {
                    register_convex_counter_owned!(
                        SNAPSHOT_EXPORT_COMPLETED_TOTAL,
                        "Count of completed exports in SnapshotExport",
                        &["backend_version", "load_description"],
                    )
                },
                "get_export_succeeded" => {
                    register_convex_counter_owned!(
                        SNAPSHOT_GET_EXPORT_SUCCEEDED_TOTAL,
                        "Count of successful GET requests for SnapshotExport",
                        &["backend_version", "load_description"],
                    )
                },
                "export_in_progress" => {
                    register_convex_counter_owned!(
                        SNAPSHOT_EXPORT_IN_PROGRESS_TOTAL,
                        "Count of in-progress exports in SnapshotExport",
                        &["backend_version", "load_description"],
                    )
                }
            },
            "CloudBackup" => btreemap!{
                "request_backup_succeeded" => {
                    register_convex_counter_owned!(
                        REQUEST_BACKUP_SUCCEEDED_TOTAL,
                        "Count of successful backup requests",
                        &["backend_version", "load_description"],
                    )
                },
                "backup_completed" => {
                    register_convex_counter_owned!(
                        BACKUP_COMPLETED_TOTAL,
                        "Count of successful backups",
                        &["backend_version", "load_description"],
                    )
                },
                "backup_timeout" => {
                    register_convex_counter_owned!(
                        BACKUP_TIMEOUT_TOTAL,
                        "Count of backup timeouts",
                        &["backend_version", "load_description"],
                    )
                },
            },
            "Search" => btreemap!{
                "search_timeout" => {
                    register_convex_counter_owned!(
                        SEARCH_TIMEOUT_TOTAL,
                        "Count of search timeouts in Search",
                        &["backend_version", "load_description"],
                    )
                }
            },
            "VectorSearch" => btreemap!{
                "vector_search_timeout" => {
                    register_convex_counter_owned!(
                        VECTOR_SEARCH_TIMEOUT_TOTAL,
                        "Count of vector search timeouts in VectorSearch",
                        &["backend_version", "load_description"],
                    )
                }
            },
        }
    });

pub fn log_event<'a>(
    mut metric_labels: Vec<MetricLabel<'a>>,
    event: &'a Event,
) -> anyhow::Result<()> {
    let Event {
        metadata:
            EventMetadata {
                name,
                scenario,
                path,
            },
        metric,
    } = event;
    if let Some(path) = path {
        metric_labels.push(MetricLabel::new("path", path));
    }

    match metric {
        Metric::Error { msg: _ } => {
            let metric = ERROR_METRICS
                .get(scenario.as_str())
                .context(format!(
                    "Scenario.name ({scenario}.{name}) missing from error metrics declarations"
                ))?
                .get(name.as_str())
                .context(format!(
                    "Scenario.name ({scenario}.{name}) missing from error metrics declarations",
                ))?;
            log_counter_with_labels(metric, 1, metric_labels);
        },
        Metric::Metric {
            r#type: metric_type,
            value,
        } => {
            match metric_type {
                MetricType::Latency => {
                    let metric = LATENCY_METRICS
                        .get(scenario.as_str())
                        .context(format!(
                            "Scenario.name ({scenario}.{name}) missing from latency metrics \
                             declarations"
                        ))?
                        .get(name.as_str())
                        .context(format!(
                            "Scenario.name ({scenario}.{name}) missing from latency metrics \
                             declarations",
                        ))?;
                    tracing::debug!("Logging {value} to {:?}", get_desc(metric));
                    log_distribution_with_labels(metric, *value, metric_labels);
                },

                MetricType::Count => {
                    let metric = COUNT_METRICS
                        .get(scenario.as_str())
                        .context(format!(
                            "Scenario.name ({scenario}.{name}) missing from count metrics \
                             declarations"
                        ))?
                        .get(name.as_str())
                        .context(format!(
                            "Scenario.name ({scenario}.{name}) missing from count metrics \
                             declarations",
                        ))?;
                    log_counter_with_labels(metric, *value as u64, metric_labels);
                },
            };
        },
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{
            BTreeMap,
            BTreeSet,
        },
        fmt::Write,
        fs,
        path::{
            Path,
            PathBuf,
        },
        sync::LazyLock,
    };

    use anyhow::Context;

    use crate::metrics::{
        COUNT_METRICS,
        ERROR_METRICS,
        LATENCY_METRICS,
    };

    #[test]
    fn test_metrics_match_scenario_runner() -> anyhow::Result<()> {
        let mut s = String::new();
        writeln!(
            &mut s,
            "// This file is automatically generated by `cargo test -p load_generator`"
        )?;
        writeln!(
            &mut s,
            "// Metrics must first be added to crates/load_generator/src/metrics.rs"
        )?;

        let mut scenario_names: BTreeSet<&str> = BTreeSet::new();
        scenario_names.extend(ERROR_METRICS.keys());
        scenario_names.extend(LATENCY_METRICS.keys());
        scenario_names.extend(COUNT_METRICS.keys());
        write!(&mut s, "export type ScenarioName =")?;
        for name in scenario_names {
            writeln!(&mut s)?;
            write!(&mut s, "  | \"{name}\"")?;
        }
        writeln!(&mut s, ";")?;

        fn write_metrics<T>(
            s: &mut String,
            varname: &str,
            metrics: &BTreeMap<&str, BTreeMap<&str, T>>,
        ) -> anyhow::Result<()> {
            write!(s, "export type {varname} =")?;
            for (scenario_name, metrics) in metrics.iter() {
                writeln!(s)?;
                write!(s, "  // {scenario_name}")?;
                for name in metrics.keys() {
                    writeln!(s)?;
                    write!(s, "  | \"{name}\"")?;
                }
            }
            writeln!(s, ";")?;
            Ok(())
        }

        write_metrics(&mut s, "ScenarioLatencyMetric", &LATENCY_METRICS)?;
        write_metrics(&mut s, "ScenarioCountMetric", &COUNT_METRICS)?;
        write_metrics(&mut s, "ScenarioError", &ERROR_METRICS)?;

        let actual = fs::read_to_string(&*SCENARIO_RUNNER_METRICS_FILE)?;
        if s != actual {
            fs::write(&*SCENARIO_RUNNER_METRICS_FILE, s)
                .context("SCENARIO_RUNNER_METRICS_FILE not found")?;
            panic!(
                "scenario-runner/metrics.ts does not match load-generator/src/metrics.rs.
                 This test will automatically update it so it will pass next time. {{s}} != \
                 {{actual}}",
            );
        }

        Ok(())
    }

    static SCENARIO_RUNNER_METRICS_FILE: LazyLock<PathBuf> = LazyLock::new(|| {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("npm-packages/scenario-runner/metrics.ts")
    });
}
