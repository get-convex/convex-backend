# Convex Docs

This website is built using [Docusaurus 2](https://docusaurus.io/), a modern
static website generator.

## Local Development

```console
just rush install
npm run dev
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

## llms.txt

This is a file that was manually generated using Firecrawl:
https://www.firecrawl.dev/blog/How-to-Create-an-llms-txt-File-for-Any-Website

You need to get an API key from Firecrawl and follow the instructions on that
blog post above.

I then did a few manual edits:

- Removed all Google Analytics references (simple regex find and replace)
- Put the home page text at the top
- Cleaned up youtube embeds output they were pretty messy.

Otherwise it generated reasonably decent output. We should eventually make this
more automated with every publish.

See
[here](https://linear.app/convex/issue/DX-1412/create-an-llmstxt-file-for-the-website-and-docs-page).
For the full background.

## Spell-checking in VS Code

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

## Updating the Agent docs

The Agent component docs are in the
[get-convex/agent repo](https://github.com/get-convex/agent/tree/main/docs). To
update them, run the following command:

```
npm run pull-agent-docs
```

This will pull the latest docs from the `main` branch and update the
`docs/agents` directory, doing some replacing of relative links back to the
agent repo for code snippets.

This is a manual process and generally only needs to be done when the agent docs
change and there is a new release of the agent package.

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
