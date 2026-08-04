# Changelog

### 3.0.0

- Add a new rule `@convex-dev/no-top-of-hour-crons` (enabled by default as a
  warning) that flags cron jobs scheduled exactly on the hour. The top of the
  hour is the busiest time on the clock for most apps, so moving background work
  off-peak (or omitting `minuteUTC` to let Convex pick a minute) helps your app
  scale. This is a major version bump because the new default-enabled rule can
  surface new warnings in existing codebases.
- The `@convex-dev/no-collect-in-query` and `@convex-dev/explicit-table-ids`
  rules now work when type-aware linting is disabled.
- When type-aware linting is disabled, error messages now recommend enabling it.

### 2.0.0

- Add a new rule `@convex-dev/no-filter-in-query` (enabled by default as a
  warning).

## 1.2.2

- Updated `@typescript-eslint/utils` to v8.58.0 to properly support ESLint v10.

## 1.2.1

- The plugin now supports ESLint v10.x.

## 1.2.0

- Added a new rule `@convex-dev/no-collect-in-query`

## 1.1.1

- Updated the dependency on `@typescript-eslint/utils` to fix a warning when
  installing the ESLint plugins with modern versions of TypeScript.

## 1.1.0

- Added a new rule `@convex-dev/explicit-table-ids`
