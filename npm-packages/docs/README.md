# Convex Docs

This powers the [Convex Docs](https://docs.convex.dev). This is an up-to-date
export of the documentation. It may not build in the open source repo - since it
was designed to run inside the Convex internal environment. Feel free to use and
modify the docs. We'll fix the docs build process eventually.

This website is built using [Docusaurus 2](https://docusaurus.io/), a modern
static website generator.

## Installation

As mentioned in the "JavaScript packages" section in the top-level `README.md`,
you'll need to use `rush` to install dependencies. Don't run `npm install`
directly, run the following Rush command instead:

```console
just rush install
```

## Local Development

```console
just run-docs
```

This command starts a local dev server and opens up a browser window. Most
changes are reflected live without having to restart the server.

If you make changes to the `convex` NPM package and want to see them reflected
in API docs, run `just rush build -t convex` and restart the server.

The command runs `npm run dev`, which will not run all checks in our presubmits.
For example, broken links are not checked. To view all errors, try building and
testing:

```console
npm run test
npm run build
```

### AI Chat

To get AI chat working when running docs locally, you need to create
`.env.local` file in this directory with a `CONVEX_URL` environment variable:

```yaml
CONVEX_URL="https://fantastic-otter-933.convex.cloud" # team: convex, project: ai-bot
```

#### Using your own AI Chat deployment

Follow the README in `npm-packages/convex-ai-chat`.

### Spell-checking in VS Code

You can enable spell checking in VS Code by installing
[Code Spell Checker](https://marketplace.visualstudio.com/items?itemName=streetsidesoftware.code-spell-checker).

## Build

```console
npm run build
```

This command generates static content into the `build` directory and can be
served using any static contents hosting service.

## Deploying to production

See [here](/ops/services/docs/release.md).

## Preview Deployment

See [here](/ops/services/docs/release.md#preview-deployment).

# Dependency notes

Typedoc plugins don't seem to work in our monorepo with Rush: they only work
when installed from npm.

We needed to update a couple, so we forked them at
https://github.com/get-convex/typedoc-plugin-markdown

Iterating on typedoc plugins is rough, typedoc implements their own module
resolution such that our rush/pnpm solution doesn't work. So to iterate I

1. cloned our typedoc-plugin-markdown fork and set a globalOverride in
   rush/pnpm-config.json
2. make changes there and did a build with yarn run build
3. removed the dependency from dashboard's package.json
4. just rush update
5. re-added the dependency to dashboard's package.json
6. just rush update
7. repeat from 2.
8. remove the globalOverridel, increment the typedoc-plugin-markdown version
   number and publish, and update docs package.json deps
