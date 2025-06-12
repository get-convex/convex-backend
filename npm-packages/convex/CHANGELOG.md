# Changelog

## Unreleased

- ConvexHttpClient mutations are now queued by default, making the
  ConvexHttpClient match the behavior of ConvexClient and ConvexReactClient.
  This makes switching between these safer.

  If you need unqueued mutations (you need to run multiple mutations
  concurrently), pass the unqueued: true option or create a separate
  ConvexHttpClient for each queue of mutations you need.

- Allow passing auth to ConvexHttpClient as an option in the constructor. This
  is appropriate for short-lived ConvexHttpClients and it more convenient for
  instantiating a client and using it in a single expression.

- Restore check that Convex functions are not imported in the browser.

  Convex functions run in a Convex deployment; including their source in a
  frontend bundle is never necessary and can unintentionally reveal
  implementation details (and even hardcoded secrets).

  This check current causes a `console.warn()` warning, but in future versions
  this will become an error. If you see the warning "Convex functions should not
  be imported in the browser" you should address this by investigating where
  this is being logged from; that's code you don't want in your frontend bundle.
  If you really want your Convex functions in the browser it's possible to
  disable this warning but this is not recommended.

- TypeScript error when async callbacks are passed to
  `mutation.withOptimisticUpdate()`: an optimistic update function is expected
  to run synchronously.

## 1.24.8

- Restore short retry timer for WebSocket reconnects initiated by an error on
  the client. This behavior was inadvertently changed in 1.24.7.

## 1.24.7

- Increase WebSocket client timeouts in general and especially for abnormal
  server errors. See
  [this incident postmortem](https://news.convex.dev/how-convex-took-down-t3-chat-june-1-2025-postmortem/)
  for more context.

## 1.24.6

- Fix another bug with new Custom JWT auth support in projects that use Convex
  backend components.

## 1.24.5

- `ConvexClient.mutation()` now accepts a third `options` argument that can
  contain an optimistic update.

## 1.24.3

- Add `.url` property to ConvexReactClient.

- Earlier errors when invalid objects are passed to `defineTable()`.

## 1.24.2

- Fix bug with new Custom JWT auth support in projects that use Convex backend
  components.

- Support larger data imports from the command line by choosing larger chunk
  size when necessary.

- Calling setAuth on a disabled ConvexClient is now an no-op.

- Add `npx convex dash` alias for dashboard command.

- Limit the number of nested query operators to 256.

## 1.24.1

- Accept `true` and `false` as values for logger in all clients, making
  disabling logs from convex functions run on a development deployment simpler:
  it's no longer necessary write your own no-op logger.

## 1.24.0

- Drop support for React 17 and remove use of `unstable_batchedUpdates` as React
  18 introduced
  [Automatic batching](https://react.dev/blog/2022/03/29/react-v18#new-feature-automatic-batching)

  If you use React 17 and choose to override the change in supported peer
  dependencies (please don't), you may notice query updates are no longer
  batched: it's possible for one Convex query update to occur on a different
  React render than another causing single frames of discrepancies in UI or
  worse, errors if you have code that relies on the client-side consistency like
  client-side joins.

  You may also notice nothing. Without batched updates some queries may be a few
  milliseconds ahead of other queries, which is still much less than the
  differences in other data fetching solutions, e.g. React Query or SWR, in
  non-batched mode.

- Remove dependency on `react-dom`, making it possible to use on "React Native
  only" projects without overriding any dependency resolution.

- New optimistic update helpers for paginated queries: three helpers
  `insertAtTop`, `insertAtBottomIfLoaded`, and `insertAtPosition`.

- The `npx convex login --login-flow paste` flag can be used to explicitly opt
  into the manual token paste login method.

- Fix MCP servers for self-hosted deployments: previously MCP CLI commands were
  attempting to contact a cloud deployment (which didn't exist) in self-hosted
  setups.

- New `compareValues` function exported from `convex/values` which matches
  Convex values semantics as implemented in backends. This function should match
  the Rust implementation in backend (and it property-tested in pursuit of
  this!) but in the event of discrepancies the Rust implementation should be
  considered authoritative.

## 1.23.0

- `npx convex dev` now supports the option of running Convex locally without an
  account

## 1.22.0

- Options for turning off MCP tools and blocking prod deployments (see
  `npx convex mcp`)
- Add `--run-sh` option to `npx convex dev`, similar to `--run` but for shell
  commands
- Add `inflightMutations` and `inflightActions` to
  `convexClient.connectionState()`

## 1.21.0

- `npx convex dev` tails logs by default. See the `--tail-logs` option for more.

- Improvement to the `.unique()` error message to print `_id`s
  [PR](https://github.com/get-convex/convex-backend/pull/59)

- Fixes to avoid race conditions in auth
  [PR](https://github.com/get-convex/convex-js/pull/29)

## 1.20.0

- Calling registered functions directly like helper functions no longer
  typechecks. See release notes for 1.18.0 for more.

- Upgrade esbuild for a sourcemap bug fix.

- Fix FieldTypeFromFieldPath to handle union of nested values and primitives.

## 1.19.5

- `npx convex mcp start` runs an MCP server. AI agents can introspect deployment
  schema (both declared and inferred) and function APIs, read data from tables,
  call functions,and write oneoff queries in JS.

## 1.19.3

- Upgrade esbuild from 0.23 to 0.25 to address security warnings about
  https://github.com/evanw/esbuild/security/advisories/GHSA-67mh-4wv8-2f99

  Convex does not use the development server functionality of esbuild which
  contains the vulnerability.

## 1.19.2

- Improved support for working with self-hosted deployments: every command that
  makes sense (e.g. not `npx convex login`) works with self-hosted deployments.

  The environment variables `CONVEX_SELF_HOSTED_URL` and
  `CONVEX_SELF_HOSTED_ADMIN_KEY` are now used to identity self-hosted
  deployments.
  https://github.com/get-convex/convex-backend/tree/main/self-hosted#self-hosting-convex
  for more.

- export the `ValidatorJSON` record types.

## 1.19.0

- Support for Local Deployments, now in beta. See
  https://docs.convex.dev/cli/local-deployments for more.

  Local deployments run your Convex dev deployment for your project on your
  local machine, which should make syncing your code faster. It also makes
  resources used during development like function calls and database bandwidth
  free, since it's your own compute resources you're using!

## 1.18.0

- Warn on direct Convex function call. This adds a console.warn whenever a
  Convex Function (mutation, query, action, internalMutation, etc.) is called
  directly

  ```ts
  export const foo = mutation(...);

  export const bar = mutation({
    args: v.any(),
    returns: v.any(),
    handler: (ctx, args) => {
      const result = await foo();
    })
  }
  ```

  because this pattern causes problems and there are straightforward
  workarounds. The problems here:

  1. Arguments and return values aren't validated despite the presence of
     validators at the function definition site.
  2. Functions called this way unexpectedly lack isolation and atomicity. Convex
     functions may be writting assuming they will run as independent
     transactions, but running these function directly breaks that assumption.
  3. Running Convex functions defined by customFunctions like triggers can cause
     deadlocks and other bad behavior.

  There are two options for how to modify your code to address the warning:

  1. Refactor it out as a helper function, then call that helper function
     directly.
  2. Use `ctx.runMutation`, `ctx.runQuery`, or `ctx.runAction()` instead of
     calling the function directly. This has more overhead (it's slower) but you
     gain isolation and atomicity because it runs as a subtransaction.

  See
  https://docs.convex.dev/understanding/best-practices/#use-helper-functions-to-write-shared-code
  for more.

  Filter to warnings in the convex dashboard logs to see if you're using this
  pattern.

  For now running functions this way only logs a warning, but this pattern is
  now deprecated and may be deleted in a future version.

- Support for Next.js 15 and
  [Clerk core 2](https://clerk.com/docs/upgrade-guides/core-2/overview):
  `@clerk/nextjs@5` and `@clerk/nextjs@6` are now known to work to Convex. Docs,
  quickstarts and templates have not yet been updated. If you're upgrading
  `@clerk/nextjs` from v4 or v5 be sure to follow the Clerk upgrade guides as
  there are many breaking changes.

- Improvements to `npx convex run`:

  - Better argument parsing with JSON5 so `{ name: "sshader" }` parses
  - support for `--identity` similar to dashboard "acting as user" feature, like
    `npx convex run --identity '{ name: "sshader" }'`
  - `npx convex run api.foo.bar` is equivalent to `npx convex run foo:bar`
  - `npx convex run convex/foo.ts:bar` is equivalent to `npx convex run foo:bar`
  - `npx convex run convex/foo.ts` is equivalent to `npx convex run foo:default`

- Allow non-JavaScript/TypeScript files in the `convex/` directory. Only .js
  etc. files will be bundled and may define Convex functions points but adding a
  temporary file like `convex/foo.tmp` will no longer break` the build.

- Fix type for FieldTypeFromFieldPath with optional objects.

- Fix types when a handler returns a promise when using return value validators
  with object syntax.

## 1.17.4

- Revert use of the identity of useAuth from Clerk to determine whether
  refreshing auth is necessary. This was causing an auth loop in Expo.

## 1.17.3

- Fetch a new JWT from Clerk when using Clerk components to change the active
  orgId or orgRole in React on the client. Any auth provider can implement this
  by returning a new `getToken` function from the `useAuth` hook passed into
  `ConvexProviderWithAuth`.

## 1.17.2

- Revert local Prettier settings change described in 1.17.1 which removed angle
  brackets in some cases where local prettier config used plugins.

- `npx convex import --replace-all` flag which behaves like the Restore
  functionality in the dashboard.

## 1.17.1

- Use local Prettier settings to format code in `convex/_generated` if found.
- Extend supported react and react-dom peerDependencies to include v19
  prereleases. This is temporary, only stable React 19 releases will be
  supported in the long term.
- Hook up Sentry reporting for local deployments, opted-into by
  `npx convex dev --local`. This telemetry will be made configurable before this
  feature is released more broadly. This is being called out here for
  transparency regarding telemetry, but this `--local` feature is not yet ready
  for general consumption. Please don't use it unless you're excited to help
  test unfinished features and willing to have errors submitted to Convex.
- Don't try to bundle .txt or .md files in the convex/ directory.
- Don't include credentials in HTTP client requests.

## 1.17.0

- Disallow extra arguments to CLI commands.
- `--component` flags for `convex import` and `convex data`.
- `--run-component` flag for `convex dev --run`
- Remove prettier-ignore-start directives from generated code.
- Fix file watcher bug where a syntax error could cause a file to stop being
  watched.
- Downgrade jwt-decode dependency back to ^3.1.2.
- Change refresh token renewal timing

## 1.16.6

- Detect TanStack Start projects and use environment variable name
  `VITE_CONVEX_URL`.

## 1.16.5

- restore `--run` flag of `convex import`

## 1.16.4

- Don't typecheck dependent components by default, add `--typecheckComponents`
  flag to typecheck.

## 1.16.3

- Fix some library typecheck errors introduced in 1.16.1. Workaround for
  previous versions is to add `"skipLibCheck": true` to the tsconfig.json in the
  convex directory.

## 1.16.2

- Change some language around components beta.

## 1.16.1

- Release components, a feature in beta. These codepaths should not be active
  unless a convex directory contains a file named `convex.config.ts`. Components
  aren't documented yet and may change significantly before general release.

## 1.16.0

- Added support for a new validator, `v.record`. This is a typed key-value
  object in TypeScript. More information can be found in the
  [docs](https://docs.convex.dev/functions/validation#record-objects).
- Upgrade esbuild from 0.17 to 0.23. It's possible to use an npm override to use
  a different version of esbuild if you need to stay on an older version,
  although changes to the esbuild API could break this in the future.

  See
  [esbuild changelog](https://github.com/evanw/esbuild/blob/main/CHANGELOG.md)
  for the full list of changes. One standout: tsconfig.json is no longer used by
  esbuild for `jsx` setting. Convex now sets it manually to
  ["automatic"](https://esbuild.github.io/api/#jsx).

## 1.15.0

- Added new command, `npx convex function-spec`, that exposes the function
  metadata (name, type, validators, visibility) of functions defined in your
  Convex deployment
- Generated code no longer includes the "Generated by convex@version" comment
- Fix issue with `convexClient.query()` so it always returns a Promise

## 1.14.0

- Updates to ConvexReactClient to work better with authentication and server
  rendering
- `npx convex init` and `npx convex reinit` have been deprecated in favor of
  `npx convex dev --configure`
- Drop support for Node.js v16, and with it drop the dependency on node-fetch.
  This removes the 'punycode' deprecation warning printed when running the CLI
  in more recent versions of Node.js.
- Support for custom claims in JWTs

## 1.13.2

- Fix `npx convex import` regression in 1.13.1

## 1.13.1

- Relax client URL validation to prepare for Convex backends accessible on
  arbitrary domain. This makes `skipConvexDeploymentUrlCheck` client option also
  no longer required for accessing deployments not hosted on the Convex BaaS.

- Fix bug where the first mutation request send over the WebSocket failing would
  not roll back the corresponding optimistic update (completedMutationId could
  be 0 which is falsey!)

- Fix bug where `codegen --init` would fail if no Convex directory existed yet.

- Action and query function wrappers now also allow validators for args
  (previously only objects were accepted) and objects for returns (previously
  only validators were accepted).

- Change `httpRouter` behavior for overlapping paths: exact matches first, then
  the longest prefix patch that matches.

## 1.13.0

- Convex queries, mutations, and actions now accept `returns:` property to
  specify a return value validator.

  Return value validators throw a runtime error (so will roll back the
  transaction in a mutation) when the value returned from a query or mutation
  does not match this validator. This is _stricter than TypeScript_ in that
  extra properties will not be allowed.

- Validator fields are now exposed: the return value of `v.object({ ... })` now
  has a `.fields` property with the validators for each property on it.

  ```
  const message = v.object({ user: v.string(), body: v.string() });
  const imageMessage = v.object({ ...message.fields, })
  ```

  These validators are also exposed on the schema at
  `schema.tables.messages.validator`

  The `Validator` export is no longer a class. It is now a discriminated union
  type of all validators where the `.kind` as the discriminator. The `Validator`
  type still has three type parameters with only the first (the TypeScript type
  enforced by the validator) required.

  The second type parameter IsOptional is no longer a boolean, is it "optional"
  or "required" now.

  These are breaking changes if you're using the two optional type parameters of
  `Validator` or doing `instanceof` checks with `Validator`! We apologize for
  the inconvenience. The same users this affects should be the ones that most
  benefit from the ability to work with validator types more directly.

- Argument validators now accept validators (object validators and union
  validators) in addition to objects with validators as properties. Return value
  validators similarly accept either validators or objects with validators as
  properties, but unlike `args:` any validator is allowed.

  Custom function wrappers (AKA middleware) should continue to work, but to
  present the same API has the builtin Convex function wrappers `query`,
  `internalQuery`, `mutation` etc. you'll need to update such code to accept
  either a validator or an object of validators. You'll likely want to update
  these anyway to support return value validators. The new `asValidator` helper
  maybe useful here.

- The default tsconfig.json created in projects when first creating the
  `convex/` directory now uses `"moduleResolution": "Bundler"`. This is just a
  better default, you
  [probably never want the previous default `"node"`/`"node10"`](https://www.typescriptlang.org/tsconfig/#moduleResolution).

## 1.12.1

- Fix bug where `npx convex deploy` and `npx convex dev` would incorrectly skip
  pushing if the only change was removing files

## 1.12.0

- `npx convex env set` works with `ENV_VAR_NAME=value` syntax

## 1.11.3

- Fix bug when filling out an empty env file
- Exclude files beginning with # from convex directory entry points
- Warn when pushing with an https.ts file
- throw if argument to Query.take() is not an integer

## 1.11.2

- Fix timestamps in npm convex logs

## 1.11.1

- Allow Clerk 5 (currently in beta) in convex peerDependencies
- Fix typechecking bug on Windows caused by the Node.js patch for CVE-2024-27980
  that makes running tsc.CMD directly no longer work
- Exclude jsonl from convex directory entry points
- Add autocomplete for project selection in new project flow
- output debugBundlePath as full bundle instead of as a single file

---

Find release notes for versions before 1.11.3 on the
[Convex Blog](https://news.convex.dev/tag/releases/).
