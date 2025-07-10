---
title: "Deploy keys"
slug: "deploy-key-types"
sidebar_position: 20
sidebar_label: "Deploy keys"
---

When you can't log in or use the CLI interactively to specify a project or
deployment, for example in a production build environment, the environment
variable `CONVEX_DEPLOY_KEY` can be set to a deploy key.

Deploy keys identify a deployment, project, or team; confer permission to take
certain actions with those resources; and can change the behavior of the convex
CLI.

### Developing locally does not require a deploy key

Running `npx convex dev` on a new machine offers the choice to log in or run
Convex locally without an account.

Logging in stores a _user token_ at `~/.convex/config.json` which is used
automatically for all CLI use going forward on that machine. This token grants
permission to push code to and read/write data from any deployment this user has
access to.

Using Convex locally without logging in
([anonymous development](/docs/cli/local-deployments-for-dev.mdx#anonymous-development))
creates a deployment locally and records this preference for this project in the
`.env.local` file in the project directory. The _admin key_ for this anonymous
backend is stored in `~/.convex/anonymous-convex-backend-state/` along with its
serialized data.

In either of these cases, there's no reason to set `CONVEX_DEPLOY_KEY`.

### Setting deploy keys

Generally deploys keys are set in a dashboard of the service that needs the key
but in most shells you can set it right before the command, like

```
CONVEX_DEPLOY_KEY='key goes here' npx convex dev
```

or set in before you run the command

```
export CONVEX_DEPLOY_KEY='key goes here'
npx convex dev
```

or add it to your .env.local file where it will be found by `npx convex`.

# Common uses of deploy keys

### Deploying from build pipelines

A _production deploy key_ specifies the production deployment of a project and
grants permissions to deploy code to it.

> `prod:qualified-jaguar-123|eyJ2...0=`

You can deploying code from a build pipeline where you can't log in (e.g.
Vercel, Netlify, Cloudflare build pipelines)

Read more about
[deploying to production](https://docs.convex.dev/production/hosting/).

### Deploying to preview deployments

A _preview deploy key_ looks like this:

> `preview:team-slug:project-slug|eyJ2...0=`

Use a preview deploy key to change the behavior of a normal `npx convex deploy`
command to deploy to a preview branch.

Read more about [preview deployments](/production/hosting/preview-deployments).

### Admin keys

An admin key provides complete control over a deployment.

An admin key might look like

> bold-hyena-681|01c2...c09c

Unlike other types of deploy key, an admin key does not require a network
connection to https://convex.dev to be used since it's a irrevocable secret
baked into the deployment when created.

These keys are used to control
[anonymous](/docs/cli/local-deployments-for-dev.mdx#anonymous-development)
Convex deployments locally without logging in, but rarely need to be set
explicitly.

Setting `CONVEX_DEPLOY_KEY` to one will cause the Convex CLI to run against that
deployment instead of offering a choice.

## Rarer types of deploy keys

### Project tokens

A _project token_ grants total control over a project to a convex CLI and
carries with it the permission to create and use development and production
deployments in that project.

> project:team-slug:project-slug|eyJ2...0=

Project tokens are obtained when a user grants an permission to use a project to
an organization via an Convex OAuth application. Actions made with the token are
on behalf of the user so if a user loses access to a project the token no longer
grant access to it.
