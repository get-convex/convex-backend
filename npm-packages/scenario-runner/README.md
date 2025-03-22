Run client-side scenarios driven by load-generator.

# Usage

Run [LoadGenerator](../../crates/load_generator/README.md). LoadGenerator will
provision a backend and start ScenarioRunner with the given parameters.

## Adding new scenarios

To add a new scenario,

1. Name the scenario and add it to `ScenarioName` and the `main` control flow in
   `index.ts`.
2. Write a class that implements the `IScenario` interface and extends the
   `Scenario` class and drop the class in the `scenarios` folder. Run this new
   scenario from the `main` control flow.
3. In LoadGenerator, add a new scenario to the `Scenario` struct.
