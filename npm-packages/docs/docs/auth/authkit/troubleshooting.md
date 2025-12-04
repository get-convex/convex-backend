---
title: "AuthKit Troubleshooting"
sidebar_label: "Troubleshooting"
sidebar_position: 30
description: "Debugging issues with AuthKit authentication with Convex"
---

## Platform not authorized

```
WorkOSPlatformNotAuthorized: Your WorkOS platform API key is not authorized to
access this team. Please ensure the API key has the correct permissions in the
WorkOS dashboard.
```

This error occurs when your WorkOS platform API key is not authorized to access
the WorkOS team associated with your Convex team.

This typically happens when the WorkOS workspace has had Convex removed.

You can contact WorkOS support to ask to restore this permission, or unlink the
current workspace and create a new one:

```bash
npx convex integration workos disconnect-team
npx convex integration workos provision-team
```

You'll need to use a different email address to create your new WorkOS Workspace
as an email address can only be associated with a single WorkOS workspace.
