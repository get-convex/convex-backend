# @convex-dev/bundle-analyzer

<!-- prettier-ignore -->
> [!NOTE]
> This package is in alpha. Commands and output shapes may change between
> releases.

Analyze your Convex project to identify ways to optimize your bundle size.

Run the analyzer from the root directory of your project:

```sh
npx @convex-dev/bundle-analyzer
```

## Requirements

- `convex/_generated/` must already exist. Run `npx convex dev --once` or
  `npx convex codegen` first. The analyzer doesn't generate code.
- The project's dependencies must be installed, since bundling resolves its
  imports from `node_modules`.

## Version matching

The analyzer will warn if the project's `convex` version differs from the
analyzer's version of `convex`. There may be slight differences in the way the
bundler behaves in different `convex` versions. A planned future feature is to
support analyzing an existing bundle, independent of `convex` version.
