---
title: ESLint rules
sidebar_position: 30
description: ESLint rules for Convex
---

ESLint rules for Convex functions enforce best practices. Let us know if there's
a rule you would find helpful!

<BetaAdmonition feature="Convex ESLint rules" verb="are" />

# Setup

## ESLint 8 (.eslintrc.js)

For ESLint 8, install these two libraries

```bash
npm i @typescript-eslint/eslint-plugin @convex-dev/eslint-plugin
```

and in .eslintrc.js:

```js
module.exports = {
  extends: [
    // Other configurations
    "plugin:@typescript-eslint/recommended",
    "plugin:@convex-dev/recommended",
  ],
  ignorePatterns: ["node_modules/", "dist/", "build/"],
};
```

## ESLint 9 (eslint.config.js)

For ESLint 9 (flat config), install just this library

```bash
npm i @convex-dev/eslint-plugin
```

and in eslint.config.js:

```bash
import convexPlugin from "@convex-dev/eslint-plugin";

export default [
  // Other configurations
  ...convexPlugin.configs.recommended
];
```

# Rules

### no-old-registered-function-syntax

Prefer object syntax for registered functions.

Convex queries, mutations, and actions can be defined with a single function or
with an object containing a handler property. Using the objects makes it
possible to add argument and return value validators, so is always preferable.

```ts
// Allowed by this rule:
export const list = query({
  handler: async (ctx) => {
    const data = await ctx.db.query("messages").collect();
    ...
  },
});

// Not allowed by this rule:
export const list = query(async (ctx) => {
  const data = await ctx.db.query("messages").collect();
  ...
});
```

### no-missing-args-validator

Use argument validators.

Convex queries, mutations, and actions can validate their arguments before
beginning to run the handler function. Besides being a concise way to validate,
the types of arguments, using argument validators enables generating more
descriptive function specs and therefore OpenAPI bindings.

```ts
// Allowed by this rule:
export const list = query({
  args: {},
  handler: async (ctx) => {
    ...
  },
});

// Allowed by this rule:
export const list = query({
  args: { channel: v.id('channel') },
  handler: async (ctx, { channel }) => {
    ...
  },
});

// Not allowed by this rule:
export const list = query({
  handler: async (ctx, { channel }: { channel: Id<"channel">}) => {
    ...
  },
});
```
