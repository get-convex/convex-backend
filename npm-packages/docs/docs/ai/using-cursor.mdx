---
title: "Using Cursor with Convex"
sidebar_position: 100
description: "Tips and best practices for using Cursor with Convex"
slug: "using-cursor"
---

[Cursor](https://cursor.com), the AI code editor, makes it easy to write and
maintain apps built with Convex. Let's walk through how to setup Cursor for the
best possible results with Convex.

## Add Convex `.cursor/rules`

To get the best results from Cursor put the model specific `.mdc` files in your
project's `.cursor/rules` directory.

- [Convex Cursor Rules](https://convex.link/convex_rules.mdc)

<video
  src="/video/showing_where_to_put_convex_rules.mp4"
  autoPlay
  loop
  controls
></video>

We're constantly working on improving the quality of these rules for Convex by
using rigorous evals. You can help by
[contributing to our evals repo](https://github.com/get-convex/convex-evals).

## Setup the Convex MCP Server

The Convex CLI comes with a
[Convex Model Context Protocol](/ai/convex-mcp-server.mdx) (MCP) server built
in. The Convex MCP server gives your AI coding agent access to the your Convex
deployment to query and optimize your project.

To get started with Cursor, open "Cursor Settings > MCP", click on "Add new
global MCP server", and add a "convex" section to "mcpServers" in the `mcp.json`
file that's opened.

```json
{
  "mcpServers": {
    "convex": {
      "command": "npx",
      "args": ["-y", "convex@latest", "mcp", "start"]
    }
  }
}
```

After adding the server, ensure the "convex" server is enabled and lit up green
(you may need to click "Disabled" to enable it). Here's an example of Cursor
configured successfully:

![Chat UI](/img/cursor-with-convex/convex_mcp_setup.webp)

If you're running into issues, confirm you're using Cursor version **0.47** or
later (check under "Cursor > About Cursor" on macOS).

Now start asking it questions like:

- Evaluate and convex schema and suggest improvements
- What are this app's public endpoints?
- Run the `my_convex_function` query

## Tips and tricks

### Install and run Convex yourself

Keeping Convex running is crucial because
[it automatically generates](https://docs.convex.dev/cli#run-the-convex-dev-server)
the client-side types. Without this, the agent can get stuck in a linting loop
since it can't access the types for the queries and mutations it created.

We recommended that you install (`npm install convex`) and run convex
(`npx convex dev`) yourself in a terminal window.

### Keep your requests small

The best results when using agentic LLMs can be found when keeping the amount of
changes you want to make small and git commit frequently. This lets you be more
specific around the context you provide the agent and it means the agent doesn't
need to do a lot of searching for context.

After each successful prompt or series of prompts it is a good idea to commit
your changes so that its simple to rollback to that point should the next prompt
cause issues.

### Update and reference your `README.md`

The agent needs context about the specific business goals for your project.
While it can infer some details from the files it reads, this becomes more
challenging as your project grows. Providing general information about your
project gives the agent a helpful head start.

Rather than including this information in each prompt, it's better to write a
comprehensive README.md file in your project root and reference it.

[Some people](https://youtu.be/2PjmPU07KNs?t=145) advocate for crafting a
Product Requirements Document (PRD), this may be a good idea for more complex
projects.

### Add Convex docs

Adding Convex docs can let you specifically refer to Convex features when
building your app.

From **`Cursor Settings`** > **`Features`** > **`Docs`** add new doc, use the
URL "https://docs.convex.dev/"

![Chat UI](/img/cursor-with-convex/adding_convex_docs.webp)

Cursor will then index all of the Convex docs for the LLM to use.

![Chat UI](/img/cursor-with-convex/indexed_docs.webp)

You can then reference those docs in your prompt with the `@Convex` symbol.

![Chat UI](/img/cursor-with-convex/reference_convex_docs.webp)

<Admonition type="tip" title="Add more Convex knowledge">

You can perform the above steps for https://stack.convex.dev/ too if you would
like to provide even more context to the agent.

</Admonition>
