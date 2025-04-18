---
title: "AI Agents"
sidebar_position: 100
description: "Building AI Agents with Convex"
---

# Building AI Agents with Convex

Convex provides a powerful platform for building AI agents through its robust
set of components.

## Why Convex for AI Agents?

Convex offers several advantages for building AI agents:

1. **Durable Execution**: Long-running workflows that survive server restarts
2. **Real-time State Management**: Reactive state updates for agent progress
3. **Built-in Persistence**: Store conversation history and agent state
4. **Parallel Processing**: Run multiple agent tasks concurrently
5. **Error Handling**: Robust retry mechanisms for API calls

## Core Components

The [Agent](https://www.convex.dev/components/agent) and
[Workflow](https://www.convex.dev/components/workflow) components can be used
together to create powerful long running agents with memory.

import { ComponentCardList } from "@site/src/components/ComponentCard";

<ComponentCardList
  items={[
    {
      title: "Agent",
      description:
        "Agents organize your AI workflows into units, with message history and vector search built in.",
      href: "https://www.convex.dev/components/agent",
    },
    {
      title: "Workflow",
      description:
        "Simplify programming long running code flows. Workflows execute durably with configurable retries and delays.",
      href: "https://www.convex.dev/components/workflow",
    },
  ]}
/>

Learn more by reading:
[AI Agents with Built-in Memory](https://stack.convex.dev/ai-agents).

Sample code:

```typescript
// Define an agent similarly to the AI SDK
const supportAgent = new Agent(components.agent, {
  chat: openai.chat("gpt-4o-mini"),
  textEmbedding: openai.embedding("text-embedding-3-small"),
  instructions: "You are a helpful assistant.",
  tools: { accountLookup, fileTicket, sendEmail },
});

// Use the agent from within a normal action:
export const createThread = action({
  args: { prompt: v.string() },
  handler: async (ctx, { prompt }) => {
    const { threadId, thread } = await supportAgent.createThread(ctx);
    const result = await thread.generateText({ prompt });
    return { threadId, text: result.text };
  },
});

// Pick up where you left off, with the same or a different agent:
export const continueThread = action({
  args: { prompt: v.string(), threadId: v.string() },
  handler: async (ctx, { prompt, threadId }) => {
    // This includes previous message history from the thread automatically.
    const { thread } = await anotherAgent.continueThread(ctx, { threadId });
    const result = await thread.generateText({ prompt });
    return result.text;
  },
});

// Or use it within a workflow, specific to a user:
export const supportAgentStep = supportAgent.asAction({ maxSteps: 10 });

const workflow = new WorkflowManager(components.workflow);
const s = internal.example; // where steps are defined

export const supportAgentWorkflow = workflow.define({
  args: { prompt: v.string(), userId: v.string(), threadId: v.string() },
  handler: async (step, { prompt, userId, threadId }) => {
    const suggestion = await step.runAction(s.supportAgentStep, {
      threadId,
      generateText: { prompt },
    });
    const polished = await step.runAction(s.adaptSuggestionForUser, {
      suggestion,
      userId,
    });
    await step.runMutation(s.sendUserMessage, {
      userId,
      message: polished.message,
    });
  },
});
```

## Other Components

Convex also provides other components to help you build reliable AI
applications.

<ComponentCardList
  items={[
    {
      title: "Persistent Text Streaming",
      description:
        "Stream text from HTTP actions while storing data in the database, enabling access after the stream ends or by other users.",
      href: "https://www.convex.dev/components/persistent-text-streaming",
    },
    {
      title: "Action Retrier",
      description:
        "Add reliability to unreliable external service calls. Retry idempotent calls with exponential backoff until success.",
      href: "https://www.convex.dev/components/retrier",
    },
    {
      title: "Workpool",
      description:
        "Create tiers of parallelism to manage and prioritize large numbers of external requests efficiently.",
      href: "https://www.convex.dev/components/workpool",
    },
  ]}
/>
