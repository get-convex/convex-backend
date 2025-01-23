# Changelog

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
