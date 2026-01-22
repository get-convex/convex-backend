---
title: "Convex MCP Server"
sidebar_position: 300
description: "Convex MCP server"
---

The Convex
[Model Context Protocol](https://docs.cursor.com/context/model-context-protocol)
(MCP) server provides several tools that allow AI agents to interact with your
Convex deployment.

## Setup

Add the following command to your MCP servers configuration:

`npx -y convex@latest mcp start`

For Cursor you can use this quick link to install:

[![Install MCP Server](https://cursor.com/deeplink/mcp-install-dark.svg)](https://cursor.com/en/install-mcp?name=convex&config=eyJjb21tYW5kIjoibnB4IC15IGNvbnZleEBsYXRlc3QgbWNwIHN0YXJ0In0%3D)

or see editor specific instructions:

- [Cursor](/ai/using-cursor.mdx#setup-the-convex-mcp-server)
- [Windsurf](/ai/using-windsurf.mdx#setup-the-convex-mcp-server)
- [VS Code](/ai/using-github-copilot.mdx#setup-the-convex-mcp-server)
- Claude Code: add the MCP server and test with
  ```bash
  claude mcp add-json convex '{"type":"stdio","command":"npx","args":["convex","mcp","start"]}'
  claude mcp get convex
  ```

## Configuration Options

The MCP server supports several command-line options to customize its behavior:

### Project Directory

By default, the MCP server can run for multiple projects, and each tool call
specifies its project directory. To run the server for a single project instead,
use:

```bash
npx -y convex@latest mcp start --project-dir /path/to/project
```

### Deployment Selection

By default, the MCP server connects to your development deployment. You can
specify a different deployment using these options:

- `--prod`: Run the MCP server on your project's production deployment (requires
  `--dangerously-enable-production-deployments`)
- `--preview-name <name>`: Run on a preview deployment with the given name
- `--deployment-name <name>`: Run on a specific deployment by name
- `--env-file <path>`: Path to a custom environment file for choosing the
  deployment (e.g., containing `CONVEX_DEPLOYMENT` or `CONVEX_SELF_HOSTED_URL`).
  Uses the same format as `.env.local` or `.env` files.

### Production Deployments

By default, the MCP server cannot access production deployments. This is a
safety measure to prevent accidental modifications to production data. If you
need to access production deployments, you must explicitly enable this:

```bash
npx -y convex@latest mcp start --dangerously-enable-production-deployments
```

<Admonition type="caution" title="Use with care">

Enabling production access allows the MCP server to read and modify data in your
production deployment. Only enable this when you specifically need to interact
with production, and be careful with any operations that modify data.

</Admonition>

### Disabling Tools

You can disable specific tools if you want to restrict what the MCP server can
do:

```bash
npx -y convex@latest mcp start --disable-tools data,run,envSet
```

Available tools that can be disabled: `data`, `envGet`, `envList`, `envRemove`,
`envSet`, `functionSpec`, `logs`, `run`, `runOneoffQuery`, `status`, `tables`

## Available Tools

### Deployment Tools

- **`status`**: Queries available deployments and returns a deployment selector
  that can be used with other tools. This is typically the first tool you'll use
  to find your Convex deployment.

### Table Tools

- **`tables`**: Lists all tables in a deployment along with their:

  - Declared schemas (if present)
  - Inferred schemas (automatically tracked by Convex)
  - Table names and metadata

- **`data`**: Allows pagination through documents in a specified table.

- **`runOneoffQuery`**: Enables writing and executing sandboxed JavaScript
  queries against your deployment's data. These queries are read-only and cannot
  modify the database.

### Function Tools

- **`functionSpec`**: Provides metadata about all deployed functions, including:

  - Function types
  - Visibility settings
  - Interface specifications

- **`run`**: Executes deployed Convex functions with provided arguments.

- **`logs`**: Fetches a chunk of recent function execution log entries, similar
  to `npx convex logs` but as structured objects.

### Environment Variable Tools

- **`envList`**: Lists all environment variables for a deployment
- **`envGet`**: Retrieves the value of a specific environment variable
- **`envSet`**: Sets a new environment variable or updates an existing one
- **`envRemove`**: Removes an environment variable from the deployment

[Read more about how to use the Convex MCP Server](https://stack.convex.dev/convex-mcp-server)
