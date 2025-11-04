---
title: "Project Configuration"
sidebar_label: "Project Configuration"
sidebar_position: 3
description:
  "Configure your Convex project for development and production deployment using
  convex.json, environment variables, and deployment settings."
---

## Local development

When you're developing locally you need two pieces of information:

1. The name of your dev deployment. This is where your functions are pushed to
   and served from. It is stored in the `CONVEX_DEPLOYMENT` environment
   variable. `npx convex dev` writes it to the `.env.local` file.
2. The URL of your dev deployment, for your client to connect to. The name of
   the variable and which file it can be read from varies between client
   frameworks. `npx convex dev` writes the URL to the `.env.local` or `.env`
   file.

## Production deployment

You should only be deploying to your production deployment once you have tested
your changes on your local deployment. When you're ready, you can deploy either
via a hosting/CI provider or from your local machine.

For a CI environment you can follow the
[hosting](/production/hosting/hosting.mdx) docs. `npx convex deploy` run by the
CI pipeline will use the `CONVEX_DEPLOY_KEY`, and the frontend build command
will use the deployment URL variable, both configured in your CI environment.

You can also deploy your backend from your local machine. `npx convex deploy`
will ask for a confirmation and then deploy to the production deployment in the
same project as your configured development `CONVEX_DEPLOYMENT`.

## `convex.json`

Additional project configuration can be specified in the `convex.json` file in
the root of your project (in the same directory as your `package.json`).

You can use the JSON schema for editor validation by adding a `$schema`
property:

```json title="convex.json"
{
  "$schema": "https://raw.githubusercontent.com/get-convex/convex-backend/refs/heads/main/npm-packages/convex/schemas/convex.schema.json"
}
```

The file supports the following configuration options:

### Changing the `convex/` folder name or location

You can choose a different name or location for the `convex/` folder via the
`functions` field. For example, Create React App doesn't allow importing from
outside the `src/` directory, so if you're using Create React App you should
have the following config:

```json title="convex.json"
{
  "$schema": "https://raw.githubusercontent.com/get-convex/convex-backend/refs/heads/main/npm-packages/convex/schemas/convex.schema.json",
  "functions": "src/convex/"
}
```

### Installing packages on the server

You can specify which packages used by Node actions should be installed on the
server, instead of being bundled, via the `node.externalPackages` field.
[Read more](/functions/bundling.mdx#external-packages).

### Importing the generated functions API via `require()` syntax

The Convex code generation can be configured to generate a CommonJS-version of
the `_generated/api.js` file via the `generateCommonJSApi` field.
[Read more](/client/javascript/node.mdx#javascript-with-commonjs-require-syntax).

### Configuring the Node.js version

You can specify which Node.js version is used by Node actions via the
`node.nodeVersion` field. The currently supported values are `"20"` and `"22"`.
[Read more](/functions/runtimes.mdx#nodejs-version-configuration).

<Admonition type="info" title="Convex version required">

To change the Node.js version used by your project, you must use the `convex`
NPM package version 1.27.0 or later.

</Admonition>

```json title="convex.json"
{
  "$schema": "https://raw.githubusercontent.com/get-convex/convex-backend/refs/heads/main/npm-packages/convex/schemas/convex.schema.json",
  "node": {
    "nodeVersion": "22"
  }
}
```

Note: This configuration is not supported when running the self-hosted Convex
backend. The node version that is specified in the
[.nvmrc](https://github.com/get-convex/convex-backend/blob/main/.nvmrc) will be
used instead.

### Using static code generation (beta)

Convex's code generation heavily relies on TypeScript's type inference. This
makes updates snappy and jump-to-definition work for the `api` and `internal`
objects, but it often slows down with large codebases.

If you're running into language server performance issues, you can instruct the
Convex CLI to generate static versions of the `_generated/api.d.ts` and
`_generated/dataModel.d.ts`:

```json title="convex.json"
{
  "$schema": "https://raw.githubusercontent.com/get-convex/convex-backend/refs/heads/main/npm-packages/convex/schemas/convex.schema.json",
  "codegen": {
    "staticApi": true,
    "staticDataModel": true
  }
}
```

This will greatly improve autocomplete and incremental typechecking performance,
but it does have some tradeoffs:

- These types only update when `convex dev` is running.
- Jump-to-definition no longer works. To find `api.example.f`, you'll need to
  manually open `convex/example.ts` and find `f`.
- Functions no longer have return type inference and will default to `v.any()`
  if they don't have a returns validator.
- [TypeScript enums](https://www.typescriptlang.org/docs/handbook/enums.html) no
  longer work in schema or API definitions. Use unions of string literal types
  instead.

This feature is currently in beta, and we'd love to improve these limitations.
Let us know if you run into any issues or have any feedback!
