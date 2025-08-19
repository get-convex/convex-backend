---
title: Tools
sidebar_label: "Tools"
sidebar_position: 500
description: "Using tool calls with the Agent component"
---

The Agent component supports tool calls, which are a way to allow an LLM to call
out to external services or functions. This can be useful for:

- Retrieving data from the database
- Writing or updating data in the database
- Searching the web for more context
- Calling an external API
- Requesting that a user takes an action before proceeding (human-in-the-loop)

## Defining tools

You can provide tools at different times:

- Agent constructor: (`new Agent(components.agent, { tools: {...} })`)
- Creating a thread: `createThread(ctx, { tools: {...} })`
- Continuing a thread: `continueThread(ctx, { tools: {...} })`
- On thread functions: `thread.generateText({ tools: {...} })`
- Outside of a thread: `supportAgent.generateText(ctx, {}, { tools: {...} })`

Specifying tools at each layer will overwrite the defaults. The tools will be
`args.tools ?? thread.tools ?? agent.options.tools`. This allows you to create
tools in a context that is convenient.

## Using tools

The Agent component will automatically handle passing tool call results back in
and re-generating if you pass `stopWhen: stepCountIs(num)` where `num > 1` to
`generateText` or `streamText`.

The tool call and result will be stored as messages in the thread associated
with the source message. See [Messages](./messages.mdx) for more details.

## Creating a tool with a Convex context

There are two ways to create a tool that has access to the Convex context.

1. Use the `createTool` function, which is a wrapper around the AI SDK's `tool`
   function.

```ts
export const ideaSearch = createTool({
  description: "Search for ideas in the database",
  args: z.object({ query: z.string().describe("The query to search for") }),
  handler: async (ctx, args, options): Promise<Array<Idea>> => {
    // ctx has agent, userId, threadId, messageId
    // as well as ActionCtx properties like auth, storage, runMutation, and runAction
    const ideas = await ctx.runQuery(api.ideas.searchIdeas, {
      query: args.query,
    });
    console.log("found ideas", ideas);
    return ideas;
  },
});
```

2. Define tools at runtime in a context with the variables you want to use.

```ts
async function createTool(ctx: ActionCtx, teamId: Id<"teams">) {
  const myTool = tool({
    description: "My tool",
    parameters: z.object({...}).describe("The arguments for the tool"),
    execute: async (args, options) => {
      return await ctx.runQuery(internal.foo.bar, args);
    },
  });
}
```

In both cases, the args and options match the underlying AI SDK's `tool`
function.

Note: it's highly recommended to use zod with `.describe` to provide details
about each parameter. This will be used to provide a description of the tool to
the LLM.

### Adding custom context to tools

It's often useful to have extra metadata in the context of a tool.

By default, the context passed to a tool is a `ToolCtx` with:

- `agent` - the Agent instance calling it
- `userId` - the user ID associated with the call, if any
- `threadId` - the thread ID, if any
- `messageId` - the message ID of the prompt message passed to generate/stream.
- Everything in `ActionCtx`, such as `auth`, `storage`, `runQuery`, etc. Note:
  in scheduled functions, workflows, etc, the auth user will be `null`.

To add more fields to the context, you can pass a custom context to the call,
such as `agent.generateText({ ...ctx, orgId: "123" })`.

You can enforce the type of the context by passing a type when constructing the
Agent.

```ts
const myAgent = new Agent<{ orgId: string }>(...);
```

Then, in your tools, you can use the `orgId` field.

```ts
type MyCtx = ToolCtx & { orgId: string };

const myTool = createTool({
  args: z.object({ ... }),
  description: "...",
  handler: async (ctx: MyCtx, args) => {
    // use ctx.orgId
  },
});
```
