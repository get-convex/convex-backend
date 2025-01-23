# Integration Tests

These tests use the `ConvexHttpClient` and `ConvexReactClient` to talk to a real
backend.

Run `just test` from this directory to run a rush build, and then run
integration tests.

Once that's done, you can run `just _test` during subsequent iterations if
you're only modifying the test suite. This will speed things up as it does not
rerun the rush build, it only spins up a backend and re-runs the test suite.

## Run individual test files

```sh
just test someFile.test.ts
just _test backend-debug someFile.test.ts
```

Remember that your file name needs to end in `.test.ts` or `.test.tsx`.

## State and Concurrency

Because all of these tests run against the same backend, there is a large risk
of leaking state between tests.

To solve this we:

1. Set Jest's `maxWorkers` to 1 so only 1 test runs at a time.
2. Have a `cleanUp` mutation that deletes all data after each test.

To ensure that `cleanUp` is complete, make sure to:

1. Add it to every new suite.
2. Put all new table names in `schema.ts:ALL_TABLE_NAMES` so we clear the table
   after every test.
