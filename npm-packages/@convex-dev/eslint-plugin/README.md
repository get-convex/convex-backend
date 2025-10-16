# @convex-dev/eslint-plugin

ESLint plugin for Convex to prevent common issues and enforce best practices for
files in the `convex/` directory.

# Setup

See [docs.convex.dev/eslint](https://docs.convex.dev/eslint).

# Contributing notes

Currently there is no `@convex-dev/eslint-config` package, but we could add one
for ESLint 8 users to make configuration slightly easier: they could use

```js
module.exports = { extends: ["@convex-dev"] };
```

and some TypeScript parser configuration could occur here.

Adding a eslint-config package only matters for ESLint 8, in ESLint 9 there's no
need for a separate package.

There are currently no rules that require TypeScript but these will be used in
the future.
