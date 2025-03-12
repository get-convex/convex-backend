---
title: "Teams"
slug: "teams"
sidebar_position: 0
---

In Convex, your projects are organized by team. Teams are used to share access
to your projects with other people. You may switch between teams or create a new
team by clicking on the name of your team located on the top of the Convex
dashboard. This will open the project selector, where you can switch teams by
clicking on the team name once again.

![Team switcher](/screenshots/team_selector.png)

You may change the name of a team or invite new members to a team by clicking on
the "Team Settings" button located on the top of the project list page.

## General

The [general page](https://dashboard.convex.dev/team/settings) allows changing
the team name and slug.

You may also delete the team from this page. You can only delete a team after
deleting all of it's projects, and removing all other team members from your
team. Deleting your team will automatically cancel your Convex subscription.

![General team settings page](/screenshots/teams_general.png)

## Team Members

Use the
[members settings page](https://dashboard.convex.dev/team/settings/members) to
invite or remove members from your team.

![Team members page](/screenshots/teams_members.png)

### Roles and permissions

Convex has two levels of control for managing access to your team, projects, and
deployments. Team-level roles control what a user can do within the team, while
project-level permissions control what a user can do within a specific project.

#### Team roles

Your team members can have one of the following roles:

- Admin
- Developer

The creator of the team is automatically assigned the Admin role. When inviting
new team members, you may select a role for them. You may also change the role
of a team member at any time.

Developers can:

- Create new projects and deployments. When a new project is created, the
  creator of the project is automatically granted the
  [Project Admin](#project-admins) role for that project.
- View existing projects, and create development and preview deployments for
  these projects. Developers may read data from production deployments, but
  cannot write to them.
- View the team's usage and billing status (such as previous and upcoming
  invoices)

Admins can do everything developers can, as well as:

- Invite new team members
- Remove members from the team
- Change the role of other team members
- Manage the team's Convex subscription and billing details.
- Change the team name and slug
- Team Admins are also implicitly granted project admin access to all projects
  within the team. See [Project Admins](#project-admins) for more information.

#### Project Admins

In addition to team roles, you may also grant admin access to individual
projects by granting team members the "Project Admin" role.

If you are a Project Admin for a given project, you may:

- Update the project name and slug
- Update the project's default environment variables
- Delete the project
- Write to production deployments

You may assign and remove the Project Admin role for multiple projects at the
same time on the member settings page. To assign or remove the Project Admin
role for multiple members at the same time, visit the
[Project Settings](/docs/dashboard/projects.md#project-settings) page instead.

## Billing

Use the [billing page](https://dashboard.convex.dev/team/settings/billing) to
upgrade your Convex subscription to a higher tier, or manage your existing
subscription.

On paid plans, you can also update your billing contact details, payment method,
and view your invoices.

[Learn more about Convex pricing](https://www.convex.dev/plans).

![Team billing page](/screenshots/teams_billing.png)

### Spending limits

When you have an active Convex subscription, you can set the spending limits for
your team on the
[billing page](https://dashboard.convex.dev/team/settings/billing):

- The **warning threshold** is only a soft limit: if it is exceeded, the team
  will be notified by email, but no other action will be taken.
- The **disable threshold** is a hard limit: if it is exceeded, all projects in
  the team will be disabled. This will cause errors to be thrown when attempting
  to run functions in your projects. You can re-enable projects by increasing or
  removing the limit.

Spending limits only apply to the resources used by your team’s projects beyond
the amounts included in your plan. The seat fees (the amount paid for each
developer in your team) are not counted towards the limits. For instance, if you
send the spending limit to $0/month, you will be billed for the seat fees only
and the projects will be disabled if you exceed the built-in resources included
in your plan.

![The team billing page with some spending limits set.](/screenshots/teams_billing_spending_limits.png)

## Usage

On the [usage page](https://dashboard.convex.dev/team/settings/usage) you can
see all the resources consumed by your team, and how you're tracking against
your plan's limits.

[Learn more about Convex pricing](https://www.convex.dev/plans).

![Team usage page](/screenshots/teams_usage.png)

All metrics are available in daily breakdowns:

![Team usage page graphs](/screenshots/teams_usage_2.png)

## Audit Log

<Admonition type="info">

The Audit Log is only available for paid teams.

</Admonition>

The [audit log page](https://dashboard.convex.dev/team/settings/audit-log) shows
all the actions taken by members within the team. This includes creating and
managing projects and deployments, inviting and removing team members, and more.

![Team audit log page](/screenshots/teams_audit_log.png)

You may also view a history of deployment-related events on the
[deployment history page](/docs/dashboard/deployments/history.md).
