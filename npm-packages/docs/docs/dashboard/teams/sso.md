---
title: "Single Sign-On (SSO)"
slug: "sso"
sidebar_position: 5
description: "Set up and manage Single Sign-On (SSO) for your Convex team"
---

<Admonition type="info">

Single Sign-On is only available on Convex Business and Enterprise.

</Admonition>

Single Sign-On (SSO) allows your team to authenticate with Convex using your
organization's identity provider (IdP). Once configured, team members can sign
in to Convex through your IdP instead of using individual credentials.

## Finding SSO settings

SSO settings are located in your team settings. To access them:

1. Click on "Team Settings" at the top of the project list page
2. Select the **Single Sign-On** tab

Or navigate directly to the
[SSO settings page](https://dashboard.convex.dev/team/settings/sso).

## Setting up SSO

To configure SSO for your team:

### 1. Enable SSO

On the Single Sign-On settings page, click **Enable SSO** to begin the setup
process.

### 2. Verify your domain

Click **Manage Domains** to verify the domain used for SSO login. Follow the
domain verification wizard to confirm ownership of your domain.

### 3. Configure your identity provider

After verifying your domain, click **Manage SSO Configuration** to set up SSO
with your identity provider of choice. Follow the instructions in the wizard to
complete the configuration.

### Renewing certificates

The SSO configuration page also allows you to renew your configuration's
certificate when it approaches expiration.

## Require Single Sign-On

Once SSO is enabled, you can optionally choose to **require** SSO for all team
members. When this setting is turned on:

- All team members must authenticate through your identity provider to access
  this team. This applies to both access via the dashboard and CLI.
- Members will not be able to sign in using other authentication methods to
  access the team

This only applies to the specific team that has SSO enabled. Members can still
use other login methods to access any other Convex teams they belong to.

To enable this, toggle the **Require SSO** option on the Single Sign-On settings
page.

## Customizing your domain policy

By default, all Convex users that sign in with your verified SSO domain will be
required to log in with SSO to use Convex If they are signing in with an email
address that uses your verified domain.

To configure a custom domain policy, such as allowing users to login with other
sign-on methods, contact Convex support.

These settings will be available for self-serve configuration in the future.
