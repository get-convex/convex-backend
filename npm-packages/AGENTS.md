# TypeScript monorepo

## Development workflow

When doing changes in `/npm-packages/<package>`:

```sh
# After each modification
just format-js

# When the change is ready
just lint-js
just rush build -t <package>

# To run a specific test file
cd npm-packages/<package>/
npm run test -- <file>
```

## Dependencies management

This project uses Rush to manage dependencies.

After modifying the dependencies of a package, run `just rush update`.

## Code organization

- **Client**: the JavaScript/React libraries for Convex,
  `npm-packages/convex/src/`
- **CLI**: the command-line tool for Convex users (`npx convex`),
  `npm-packages/convex/src/cli/`
- **Dashboard**: the web user interface for Convex users
  - `npm-packages/dashboard/` for the Convex Cloud dashboard
    (https://dashboard.convex.dev/)
  - `npm-packages/dashboard-self-hosted/` for the self-hosted build of the
    dashboard
  - `npm-packages/dashboard-common/` for code that’s common to both dashboard
    versions
  - `npm-packages/@convex-dev/design-system/` for UI elements
  - `npm-packages/system-udfs/` for Convex functions the dashboard/CLI can call
    on deployments
- **Docs**: public docs at https://docs.convex.dev/, `npm-packages/docs/`
