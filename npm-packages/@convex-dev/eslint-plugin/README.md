# @convex-dev/eslint-plugin

ESLint plugin for Convex to prevent common issues and enforce best practices for
files in the `convex/` directory.

# Setup

### ESLint 8 (`.eslintrc.js`)

```bash
npm i @typescript-eslint/eslint-plugin @convex-dev/eslint-plugin
```

and add these two in `.eslintrc.js`:

```js
module.exports = {
  extends: [
    "plugin:@typescript-eslint/recommended",
    "plugin:@convex-dev/recommended",
  ],
  ignorePatterns: ["node_modules/", "dist/", "build/"],
};
```

### ESLint 9 (`eslint.config.js`)

```bash
npm i @convex-dev/eslint-plugin
```

In `eslint.config.js`:

```js
import convexPlugin from "@convex-dev/eslint-plugin";

export default [
  // Other configurations
  ...convexPlugin.configs.recommended,
];
```

### Next.js

For `next lint` to run eslint on your convex directory you need to add that
directory to the default set of pages, app, components, lib, and src. Add this
section to your `next.config.ts`:

```ts
const nextConfig: NextConfig = {
  /* other options here */

  eslint: {
    dirs: ["pages", "app", "components", "lib", "src", "convex"],
  },
};
```

## Setup with a custom Convex directory location

If your Convex directory isn't called `convex`, you need to customize a bit.
This setup may change with new versions of the Convex ESLint plugin.

### ESLint 8 (`eslintrc.js`), custom Convex directory location

In `eslintrc.js`, add:

```js
module.exports = {
  extends: ["plugin:@convex-dev/recommendedRulesCustomConvexDirectoryLocation"],

  overrides: [
    {
      files: ["**/myCustomConvexDirectoryName/**/*.ts"],
      extends: [
        "plugin:@convex-dev/eslint-plugin/recommendedRulesCustomConvexDirectoryLocation",
      ],
    },
  ],
};
```

### ESLint 9 (`eslint.config.js`), custom Convex directory location

In `eslint.config.js`, add:

```js
import convexPlugin from "@convex-dev/eslint-plugin";

const recommendedConfig = convexPlugin.configs.recommended[0];
const recommendedRules = recommendedConfig.rules;

export default [
  // Other configurations go here...

  // Custom configuration with modified directory pattern
  {
    files: ["**/myconvex/**/*.ts"],
    plugins: {
      "@convex-dev": convexPlugin,
    },
    rules: recommendedRules,
  },
];
```

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
