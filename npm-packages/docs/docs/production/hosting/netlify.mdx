---
title: "Using Convex with Netlify"
sidebar_label: "Netlify"
description: "Host your frontend on Netlify and your backend on Convex"
sidebar_position: 20
---

Hosting your Convex app on Netlify allows you to automatically re-deploy both
your backend and your frontend whenever you push your code.

## Deploying to Netlify

This guide assumes you already have a working React app with Convex. If not
follow the [Convex React Quickstart](/quickstart/react.mdx) first. Then:

<StepByStep>
  <Step title="Create a Netlify account">
    If you haven't done so, create a [Netlify](https://netlify.com) account.
    This is free for small projects and should take less than a minute to set
    up.

    <></>

  </Step>
  <Step title="Link your project on Netlify">
    Create a Netlify project at https://app.netlify.com/start and link it to the
    source code repository for your project on GitHub or other Git platform.

    <div className="screenshot-border">
      ![Netlify import project](/screenshots/netlify_import.png)
    </div>

  </Step>
  <Step title="Override the Build command">
    Override the _Build command_ to be
    `npx convex deploy --cmd 'npm run build'`.

    If your project lives in a subdirectory of your repository you'll
    also need to change _Base directory_ in Netlify accordingly.

    <div className="screenshot-border">
      ![Netlify build settings](/screenshots/netlify_build_settings.png)
    </div>

  </Step>
  <Step title="Set up the CONVEX_DEPLOY_KEY environment variable">
    On your [Convex Dashboard](https://dashboard.convex.dev/)
    go to your project's _Settings_ page. Click the _Generate_ button to generate a **Production** deploy key.
    Then click the copy button to copy the key.

    In Netlify, click _Add environment variables_ and _New variable_.

    Create an environment variable `CONVEX_DEPLOY_KEY` and paste
    in your deploy key.

    <div className="screenshot-border">
      ![Netlify environment variable CONVEX_DEPLOY_KEY](/screenshots/netlify_prod_deploy_key.png)
    </div>

  </Step>
  <Step title="Deploy your site">
    Now click the _Deploy_ button and your work here is done!

    <></>

  </Step>

</StepByStep>

Netlify will automatically publish your site to a URL
`https://<site-name>.netlify.app` listed at the top of the site overview page.
Every time you push to your git repository, Netlify will automatically deploy
your Convex functions and publish your site changes.

<Admonition type="info" title="Using a Custom Domain?">
  If you're using a custom domain to serve your Convex functions, you'll need
  additional configuration. See [Custom
  Domains](/production/hosting/custom.mdx#hosting-with-a-custom-domain) for more
  information.
</Admonition>
### How it works

In Netlify, we overrode the _Build Command_ to be
`npx convex deploy --cmd 'npm run build'`.

`npx convex deploy` will read `CONVEX_DEPLOY_KEY` from the environment and use
it to set the `CONVEX_URL` (or similarly named) environment variable to point to
your **production** deployment.

Your frontend framework of choice invoked by `npm run build` will read the
`CONVEX_URL` environment variable and point your deployed site (via
`ConvexReactClient`) at your **production** deployment.

Finally, `npx convex deploy` will push your Convex functions to your production
deployment.

Now, your production deployment has your newest functions and your app is
configured to connect to it.

You can use `--cmd-url-env-var-name` to customize the variable name used by your
frontend code if the `deploy` command cannot infer it, like

```sh
npx convex deploy --cmd-url-env-var-name CUSTOM_CONVEX_URL --cmd 'npm run build'
```

## Authentication

You will want to configure your [authentication](/auth.mdx) provider (Clerk,
Auth0 or other) to accept your production `<site-name>.netlify.app` URL.

## Deploy Previews

Netlify's Deploy Previews allow you to preview changes to your app before
they're merged in. In order to preview both changes to frontend code and Convex
functions, you can set up
[Convex preview deployments](/production/hosting/preview-deployments.mdx).

This will create a fresh Convex backend for each preview and leave your
production and development deployments unaffected.

This assumes you have already followed the steps in
[Deploying to Netlify](#deploying-to-netlify) above.

<StepByStep>
  <Step title="Set up the CONVEX_DEPLOY_KEY environment variable">
    On your [Convex Dashboard](https://dashboard.convex.dev/)
    go to your project's _Settings_ page. Click the _Generate Preview Deploy Key_ button to generate a **Preview** deploy key.
    Then click the copy button to copy the key.

    In Netlify, click _Site configuration_ > _Environment variables_. Edit your existing `CONVEX_DEPLOY_KEY` environment variable.
    Select _Different value for each deploy context_ and paste the key under _Deploy Previews_.


    <div className="screenshot-border">
      ![Netlify environment variable CONVEX_DEPLOY_KEY](/screenshots/netlify_preview_deploy_key.png)
    </div>

  </Step>
  <Step title="(optional) Set up default environment variables">
    If your app depends on certain Convex environment variables, you can set up [default
    environment variables](/production/environment-variables.mdx#project-environment-variable-defaults) for preview and development deployments in your project.
    <div className="screenshot-border">
      ![Project Default Environment Variables](/screenshots/project_default_environment_variables.png)
    </div>
  </Step>

<Step title="(optional) Run a function to set up initial data">
  Deploy Previews run against fresh Convex backends, which do not share data
  with development or production Convex deployments. You can call a Convex
  function to set up data by adding `--preview-run 'functionName'` to the `npx
  convex deploy` command. This function will only be run for preview deployments, and will be ignored
  when deploying to production.

```sh title="Netlify > Site configuration > Build & deploy > Build settings > Build command"
npx convex deploy --cmd 'npm run build' --preview-run 'functionName'
```

</Step>

  <Step title="Now test out creating a PR and generating a Deploy Preview!">

    You can find the Convex deployment for your branch in the Convex dashboard.
    <div className="screenshot-border">
      ![Preview Deployment in Deployment Picker](/screenshots/preview_deployment_deployment_picker.png)
    </div>

  </Step>

</StepByStep>

### How it works

For Deploy Previews, `npx convex deploy` will read `CONVEX_DEPLOY_KEY` from the
environment, and use it to create a Convex deployment associated with the Git
branch name for the Deploy Preview. It will set the `CONVEX_URL` (or similarly
named) environment variable to point to the new Convex deployment.

Your frontend framework of choice invoked by `npm run build` will read the
`CONVEX_URL` environment variable and point your deployed site (via
`ConvexReactClient`) at the Convex preview deployment.

Finally, `npx convex deploy` will push your Convex functions to the preview
deployment and run the `--preview-run` function (if provided). This deployment
has separate functions, data, crons and all other configuration from any other
deployments.

`npx convex deploy` will infer the Git branch name for Vercel, Netlify, GitHub,
and GitLab environments, but the `--preview-create` option can be used to
customize the name associated with the newly created deployment.

Production deployments will work exactly the same as before.
