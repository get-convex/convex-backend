use std::{
    collections::BTreeSet,
    env,
    process,
    time::Duration,
};

use serde_json::json;
use wasm_runtime::benchmark_support::{
    artifact_output_dir,
    load_prepared_fixture,
    measure_fast_requests,
    prepare_fixture,
    setup_timings,
    BenchmarkScenario,
    RequestBenchmarkResult,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut fixtures = BTreeSet::new();
    let mut iterations = 10_usize;
    let mut concurrency = 1_usize;
    let mut workers = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1);
    let mut cpu_work = 500_000_u64;
    let mut include_setup = false;
    let mut prepare_only = false;
    let mut use_prepared = false;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--fixture" => {
                let fixture = args
                    .next()
                    .ok_or_else(|| "--fixture requires a value".to_owned())?;
                fixtures.insert(fixture);
            },
            "--iterations" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--iterations requires a value".to_owned())?;
                iterations = value
                    .parse::<usize>()
                    .map_err(|error| format!("invalid --iterations value {value:?}: {error}"))?;
            },
            "--concurrency" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--concurrency requires a value".to_owned())?;
                concurrency = value
                    .parse::<usize>()
                    .map_err(|error| format!("invalid --concurrency value {value:?}: {error}"))?;
                if concurrency == 0 {
                    return Err("--concurrency must be at least 1".to_owned());
                }
            },
            "--workers" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--workers requires a value".to_owned())?;
                workers = value
                    .parse::<usize>()
                    .map_err(|error| format!("invalid --workers value {value:?}: {error}"))?;
                if workers == 0 {
                    return Err("--workers must be at least 1".to_owned());
                }
            },
            "--cpu-work" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--cpu-work requires a value".to_owned())?;
                cpu_work = value
                    .parse::<u64>()
                    .map_err(|error| format!("invalid --cpu-work value {value:?}: {error}"))?;
            },
            "--include-setup" => {
                include_setup = true;
            },
            "--prepare-only" => {
                prepare_only = true;
            },
            "--use-prepared" => {
                use_prepared = true;
            },
            "--help" | "-h" => {
                print_help();
                return Ok(());
            },
            other => {
                return Err(format!("unknown argument: {other}"));
            },
        }
    }

    if fixtures.is_empty() {
        fixtures.extend([
            "cpu-heavy".to_owned(),
            "light-load".to_owned(),
            "heavy-globals".to_owned(),
            "async-db".to_owned(),
        ]);
    }

    let mut fixture_summaries = Vec::new();

    for fixture in fixtures {
        let artifacts = if use_prepared {
            load_prepared_fixture(&fixture)?
        } else {
            prepare_fixture(&fixture)?
        };
        let mut cases = Vec::new();

        if prepare_only {
            let setup = setup_timings(&artifacts);
            fixture_summaries.push(json!({
                "fixture": fixture,
                "artifactDir": artifact_output_dir(&artifacts.fixture),
                "bundleMs": millis(setup.bundle),
                "guestBuildMs": millis(setup.guest_build),
                "preinitMs": millis(setup.preinit),
            }));
            continue;
        }

        match fixture.as_str() {
            "light-load" => {
                cases.push(measure_fast_requests(
                    &artifacts,
                    "fast_sync",
                    BenchmarkScenario::Sync {
                        handler: "sum",
                        args_json: r#"[[1,2,3,4,5]]"#,
                    },
                    iterations,
                    concurrency,
                    workers,
                )?);
            },
            "cpu-heavy" => {
                cases.push(measure_fast_requests(
                    &artifacts,
                    "cpu_burn",
                    BenchmarkScenario::CpuHeavy { work: cpu_work },
                    iterations,
                    concurrency,
                    workers,
                )?);
            },
            "heavy-globals" => {
                cases.push(measure_fast_requests(
                    &artifacts,
                    "fast_sync",
                    BenchmarkScenario::Sync {
                        handler: "summarizeCatalog",
                        args_json: r#"[]"#,
                    },
                    iterations,
                    concurrency,
                    workers,
                )?);
            },
            "async-db" => {
                cases.push(measure_fast_requests(
                    &artifacts,
                    "async_round_trip",
                    BenchmarkScenario::AsyncRoundTrip,
                    iterations,
                    concurrency,
                    workers,
                )?);
                cases.push(measure_fast_requests(
                    &artifacts,
                    "async_fanout",
                    BenchmarkScenario::AsyncFanout,
                    iterations,
                    concurrency,
                    workers,
                )?);
            },
            other => return Err(format!("unsupported fixture: {other}")),
        }

        let mut summary = json!({
            "fixture": fixture,
            "cases": cases.into_iter().map(case_json).collect::<Vec<_>>(),
        });

        if include_setup {
            let setup = setup_timings(&artifacts);
            summary["setup"] = json!({
                "artifactDir": artifacts.output_dir,
                "bundleMs": millis(setup.bundle),
                "guestBuildMs": millis(setup.guest_build),
                "preinitMs": millis(setup.preinit),
            });
        }

        fixture_summaries.push(summary);
    }

    let output = json!({
        "iterations": iterations,
        "concurrency": concurrency,
        "workers": workers,
        "cpuWork": cpu_work,
        "mode": if prepare_only { "prepare-only" } else { "request-only" },
        "prepared": use_prepared,
        "hostProfile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "guestProfile": "release",
        "fixtures": fixture_summaries,
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&output).expect("benchmark output should serialize")
    );

    Ok(())
}

fn millis(duration: std::time::Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn case_json(result: RequestBenchmarkResult) -> serde_json::Value {
    let request_total = result.instantiate_total + result.invoke_total;
    json!({
        "case": result.case,
        "iterationsPerWorker": result.iterations_per_worker,
        "concurrency": result.concurrency,
        "workers": result.workers,
        "requests": result.requests,
        "wallMs": millis(result.wall_total),
        "throughputRps": rate(result.requests, result.wall_total),
        "instantiateMs": duration_breakdown(result.instantiate_total, result.requests),
        "invokeMs": duration_breakdown(result.invoke_total, result.requests),
        "requestMs": duration_breakdown(request_total, result.requests),
    })
}

fn duration_breakdown(total: Duration, requests: usize) -> serde_json::Value {
    json!({
        "total": millis(total),
        "average": millis(avg_duration(total, requests)),
    })
}

fn avg_duration(total: Duration, requests: usize) -> Duration {
    Duration::from_secs_f64(total.as_secs_f64() / requests as f64)
}

fn rate(requests: usize, total: Duration) -> f64 {
    if total.is_zero() {
        return 0.0;
    }
    requests as f64 / total.as_secs_f64()
}

fn print_help() {
    println!(
        "usage: cargo run --release --bin benchmark -- [--iterations N] [--concurrency N] \
         [--workers N] [--fixture NAME ...] [--cpu-work N] [--include-setup] [--prepare-only] \
         [--use-prepared]"
    );
}
