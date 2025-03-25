# LoadGenerator

LoadGenerator is a powerful load testing and benchmarking tool designed to
evaluate Convex's performance under various workloads. It helps measure:

- Function latency and throughput
- Performance under different load patterns
- Overall system stability

The tool works by:

0. Provisioning a Convex instance (or pointing at an existing Convex instance).
1. Sending predefined or custom scenarios to
   [ScenarioRunner](../../npm-packages/scenario-runner/README.md)
2. Collecting performance metrics from the executed scenarios
3. Generating detailed statistics reports with latency metrics
4. Optionally sending metrics to production monitoring systems (e.g., Datadog)

### Architecture

```
  ┌──────────┐
  │  Stats   │
  │  Report  │        ┌────────────────┐           ┌───────────────────┐           ┌───────────────┐
  └──────────┘        │                │           │                   │ queries,  │               │
        ▲             │                │ Scenarios │                   │ mutations │               │
        └─────────┬───│ LoadGenerator  │──────────▶│  ScenarioRunner   │──────────▶│    Backend    │
┌──────────────┐  │   │                │◀──────────│                   │◀──────────│               │
│              │  │   │                │  Events   │                   │           │               │
│   Metrics    │  │   └────────────────┘           └───────────────────┘           └───────────────┘
│  Collector   │◀─┘
│(e.g. Datadog)│
│              │
└──────────────┘
```

## Usage

From the root `convex` directory, run the following for usage instructions:

```shell
cargo run -p load_generator --bin load-generator -- --help
```

See the Justfile for details on running preconfigured workloads automatically.

If you want tracing, make sure to add `RUST_LOG=info` before your run command.

## Instructions for using LoadGenerator to benchmark self-hosted Convex

1. Push scenario-runner functions to your self-hosted Convex backend. Do not run
   this against your production instance! This will replace your functions. Use
   a separate Convex backend set up for testing only.

   ```sh
   cd npm-packages/scenario-runner
   npx convex deploy --admin-key=<your-admin-key> --url=<your-backend-url>
   ```

2. Run LoadGenerator against your self-hosted Convex backend. See the
   `workloads` directory for example workloads. You can specify a rate to run
   each scenario at, in number of requests per second (see
   [workloads/prod.json](workloads/prod.json)), or the number of threads to run
   continuous requests on in benchmark mode (see
   [workloads/benchmark_query.json](workloads/benchmark_query.json)).

   ```sh
   cd ../../crates/load_generator
   just self-hosted crates/load_generator/workloads/<your-workload>.json  --existing-instance-url <your-backend-url> --existing-instance-admin-key <your-admin-key>
   ```

## Writing custom scenarios

You can also write your own Convex functions to run with LoadGenerator by adding
them to the `convex` folder in
[`npm-packages/scenario-runner`](../../npm-packages/scenario-runner/convex/).
Make sure the function name takes no arguments, then drop it in your workload
config as a `RunFunction` scenario, push your functions, and run LoadGenerator
with the path to your new workload config!

```json
{
  "name": "your_new_workload",
  "scenarios": [
    {
      "name": "RunFunction",
      "path": "<your-new-module>:<your-function-name>",
      "fn_type": "mutation", // or "query" or "action"
      "rate": 5 // whatever rate you'd like, or benchmark threads
    }
  ]
}
```
