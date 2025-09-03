---
title: Workflows
sidebar_label: "Workflows"
sidebar_position: 800
description: "Defining long-lived workflows for the Agent component"
---

Agentic Workflows can be decomposed into two elements:

1. Prompting an LLM (including message history, context, etc.).
2. Deciding what to do with the LLM's response.

We generally call them workflows when there are multiple steps involved, they
involve dynamically deciding what to do next, are long-lived, or have a mix of
business logic and LLM calls.

Tool calls and MCP come into play when the LLM's response is a specific request
for an action to take. The list of available tools and result of the calls are
used in the prompt to the LLM.

One especially powerful form of Workflows are those that can be modeled as
[durable functions](https://stack.convex.dev/durable-workflows-and-strong-guarantees)
that can be long-lived, survive server restarts, and have strong guarantees
around retrying, idempotency, and completing.

The simplest version of this could be doing a couple pre-defined steps, such as
first getting the weather forecast, then getting fashion advice based on the
weather. For a code example, see
[workflows/chaining.ts](https://github.com/get-convex/agent/blob/main/example/convex/workflows/chaining.ts).

```ts
export const getAdvice = action({
  args: { location: v.string(), threadId: v.string() },
  handler: async (ctx, { location, threadId }) => {
    // This uses tool calls to get the weather forecast.
    await weatherAgent.generateText(
      ctx,
      { threadId },
      { prompt: `What is the weather in ${location}?` },
    );
    // This includes previous message history from the thread automatically and
    // uses tool calls to get user-specific fashion advice.
    await fashionAgent.generateText(
      ctx,
      { threadId },
      { prompt: `What should I wear based on the weather?` },
    );
    // We don't need to return anything, since the messages are saved
    // automatically and clients will get the response via subscriptions.
  },
});
```

## Building reliable workflows

One common pitfall when working with LLMs is their unreliability. API providers
have outages, and LLMs can be flaky. To build reliable workflows, you often need
three properties:

1. Reliable retries
2. Load balancing
3. Durability and idempotency for multi-step workflows

Thankfully there are Convex components to leverage for these properties.

### Retries

By default, Convex mutations have these properties by default. However, calling
LLMs require side-effects and using the network calls, which necessitates using
actions. If you are only worried about retries, you can use the
[Action Retrier](https://convex.dev/components/retrier) component.

However, keep reading, as the [Workpool](https://convex.dev/components/workpool)
and [Workflow](https://convex.dev/components/workflow) components provide more
robust solutions, including retries.

### Load balancing

With long-running actions in a serverless environment, you may consume a lot of
resources. And with tasks like ingesting data for RAG or other spiky workloads,
there's a risk of running out of resources. To mitigate this, you can use the
[Workpool](https://convex.dev/components/workpool) component. You can set a
limit on the number of concurrent workers and add work asynchronously, with
configurable retries and a callback to handle eventual success / failure.

However, if you also want to manage multi-step workflows, you should use the
[Workflow](https://convex.dev/components/workflow) component, which also
provides retries and load balancing out of the box.

### Durability and idempotency for multi-step workflows

When doing multi-step workflows that can fail mid-way, you need to ensure that
the workflow can be resumed from where it left off, without duplicating work.
The [Workflow](https://convex.dev/components/workflow) builds on the
[Workpool](https://convex.dev/components/workpool) to provide durable execution
of long running functions with retries and delays.

Each step in the workflow is run, with the result recorded. Even if the server
fails mid-way, it will resume with the latest incomplete step, with configurable
retry settings.

## Using the Workflow component for long-lived durable workflows

The [Workflow component](https://convex.dev/components/workflow) is a great way
to build long-lived, durable workflows. It handles retries and guarantees of
eventually completing, surviving server restarts, and more. Read more about
durable workflows in
[this Stack post](https://stack.convex.dev/durable-workflows-and-strong-guarantees).

To use the agent alongside workflows, you can run individual idempotent steps
that the workflow can run, each with configurable retries, with guarantees that
the workflow will eventually complete. Even if the server crashes mid-workflow,
the workflow will pick up from where it left off and run the next step. If a
step fails and isn't caught by the workflow, the workflow's onComplete handler
will get the error result.

### Exposing the agent as Convex actions

You can expose the agent's capabilities as Convex functions to be used as steps
in a workflow.

To create a thread as a standalone mutation, similar to `createThread`:

```ts
export const createThread = supportAgent.createThreadMutation();
```

For an action that generates text in a thread, similar to `thread.generateText`:

```ts
export const getSupport = supportAgent.asTextAction({
  stopWhen: stepCountIs(10),
});
```

You can also expose a standalone action that generates an object.

```ts
export const getStructuredSupport = supportAgent.asObjectAction({
  schema: z.object({
    analysis: z.string().describe("A detailed analysis of the user's request."),
    suggestion: z.string().describe("A suggested action to take."),
  }),
});
```

To save messages explicitly as a mutation, similar to `agent.saveMessages`:

```ts
export const saveMessages = supportAgent.asSaveMessagesMutation();
```

This is useful for idempotency, as you can first create the user's message, then
generate a response in an unreliable action with retries, passing in the
existing messageId instead of a prompt.

### Using the agent actions within a workflow

You can use the [Workflow component](https://convex.dev/components/workflow) to
run agent flows. It handles retries and guarantees of eventually completing,
surviving server restarts, and more. Read more about durable workflows
[in this Stack post](https://stack.convex.dev/durable-workflows-and-strong-guarantees).

```ts
const workflow = new WorkflowManager(components.workflow);

export const supportAgentWorkflow = workflow.define({
  args: { prompt: v.string(), userId: v.string() },
  handler: async (step, { prompt, userId }) => {
    const { threadId } = await step.runMutation(internal.example.createThread, {
      userId,
      title: "Support Request",
    });
    const suggestion = await step.runAction(internal.example.getSupport, {
      threadId,
      userId,
      prompt,
    });
    const { object } = await step.runAction(
      internal.example.getStructuredSupport,
      {
        userId,
        message: suggestion,
      },
    );
    await step.runMutation(internal.example.sendUserMessage, {
      userId,
      message: object.suggestion,
    });
  },
});
```

See the code in
[workflows/chaining.ts](https://github.com/get-convex/agent/blob/main/example/convex/workflows/chaining.ts).

## Complex workflow patterns

While there is only an example of a simple workflow here, there are many complex
patterns that can be built with the Agent component:

- Dynamic routing to agents based on an LLM call or vector search
- Fanning out to LLM calls, then combining the results
- Orchestrating multiple agents
- Cycles of Reasoning and Acting (ReAct)
- Modeling a network of agents messaging each other
- Workflows that can be paused and resumed

import { ComponentCardList } from "@site/src/components/ComponentCard";

<ComponentCardList
  items={[
    {
      title: "Action Retrier",
      description:
        "Add reliability to unreliable external service calls. Retry idempotent calls with exponential backoff until success.",
      href: "https://www.convex.dev/components/retrier",
    },
    {
      title: "Workpool",
      description:
        "Builds on the Action Retrier to provide parallelism limits and retries to manage large numbers of external requests efficiently.",
      href: "https://www.convex.dev/components/workpool",
    },
    {
      title: "Workflow",
      description:
        "Builds on the Workpool to provide durable execution of long running functions with retries and delays.",
      href: "https://www.convex.dev/components/workflow",
    },
  ]}
/>
