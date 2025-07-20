---
title: "Projects"
slug: "projects"
sidebar_position: 10
description: "Create and manage Convex projects, settings, and deployments"
---

![Project settings](/screenshots/projects.png)

A project corresponds to a codebase that uses Convex, which contains a
production deployment and one personal deployment for each team member.

Clicking on a project in the [landing page](https://dashboard.convex.dev) will
redirect you to project details.

## Creating a project

Projects can be created from the dashboard or from the
[CLI](/cli.md#create-a-new-project). To create a project from the dashboard
click on the Create Project button.

## Project Settings

You can access project-level settings by clicking on the triple-dot `⋮` button
on each Project card on the Projects page.

![Project card menu](/screenshots/project_menu.png)

On the [Project Settings page](https://dashboard.convex.dev/project/settings),
you can:

- Update your project's name and slug.
- Manage the project's Admins. See
  [Roles and Permissions](/dashboard/teams.md#roles-and-permissions) for more
  details.
- View the amount of [usage metrics](/dashboard/teams.md#usage) your project has
  consumed.
- Add [custom domains](/production/hosting/custom.mdx#custom-domains) for your
  production deployment
- Generate deploy keys for your production and preview deployments.
- Create and edit
  [default environment variables](/production/environment-variables.mdx#project-environment-variable-defaults).
- View instructions to regain access to your project, should you lose track of
  your `CONVEX_DEPLOYMENT` config.
- Permanently delete the project.

![Project settings](/screenshots/project_settings.png)

## Deleting projects

To delete a project, click on the triple-dot `⋮` button on the Project card and
select "Delete". You may also delete your project from the Project Settings
page.

Once a project is deleted, it cannot be recovered. All deployments and data
associated with the project will be permanently removed. When deleting a project
from the dashboard, you will be asked to confirm the deletion. Projects with
activity in the production deployment will have additional confirmation steps to
prevent accidental deletion.

![Delete project](/screenshots/project_delete.png)
