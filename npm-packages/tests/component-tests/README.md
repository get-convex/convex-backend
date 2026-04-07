# component-tests

This directory is used for application-level component tests.

The application-level component tests go through the push codepath to analyze,
upload modules, and set up the component model in system tables in the database.

These tests simulate a push by reading from pre-written `StartPushRequest`s from
the `isolate/build.rs` script, which runs
`npx convex deploy --start-push-request` for convex projects in the `projects`
directory in this directory.

## Adding new convex projects to test component layouts

To test a new component layout,

1. Add a project to the `projects` directory. The project should have a `convex`
   folder inside and `convex.config.ts` (If your test does not use components,
   use `udf-tests`).
2. Add the project to `COMPONENT_TESTS_PROJECTS` in `isolate/build.rs.
3. Add the project to `rush.json`.
4. Run `just rush update`.
5. Write your tests in `application/src/tests/components.rs`.

## Adding new components

To test a new component, add the component to this directory (`component-tests`)
and `COMPONENTS` in `isolate/build.rs`. Use it in a project in the `projects`
directory by installing it in the project's `convex.config.ts`.
