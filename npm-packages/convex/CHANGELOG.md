# Changelog

## Unpublished

- Drop support for Node.js v16, and with it drop the dependency on node-fetch.
  This removes the 'punycode' deprecation warning printed when running the CLI
  in more recent versions of Node.js.

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
