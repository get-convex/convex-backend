# load-generator

LoadGenerator is a tool to send scenarios based on a workload to
[ScenarioRunner](../../npm-packages/scenario-runner/README.md) and generate a
stats report with latency metrics from Events received from `ScenarioRunner`. In
production, LoadGenerator sends these metrics to our metrics collector and can
optionally print out a stats report upon completion.

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
