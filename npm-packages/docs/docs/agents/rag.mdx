---
title: RAG (Retrieval-Augmented Generation) with the Agent component
sidebar_label: "RAG"
sidebar_position: 700
description: "Examples of how to use RAG with the Convex Agent component"
---

The Agent component has built-in capabilities to search message history with
hybrid text & vector search. You can also use the RAG component to use other
data to search for context.

## What is RAG?

Retrieval-Augmented Generation (RAG) is a technique that allows an LLM to search
through custom knowledge bases to answer questions.

RAG combines the power of Large Language Models (LLMs) with knowledge retrieval.
Instead of relying solely on the model's training data, RAG allows your AI to:

- Search through custom documents and knowledge bases
- Retrieve relevant context for answering questions
- Provide more accurate, up-to-date, and domain-specific responses
- Cite sources and explain what information was used

## RAG Component

<div className="center-image" style={{ maxWidth: "560px" }}>
  <iframe
    width="560"
    height="315"
    src="https://www.youtube.com/embed/dGmtAmdAaFs?si=ce-M8pt6EWDZ8tfd"
    title="RAG Component YouTube Video"
    frameborder="0"
    allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
    referrerpolicy="strict-origin-when-cross-origin"
    allowfullscreen
  ></iframe>
</div>

The RAG component is a Convex component that allows you to add data that you can
search. It breaks up the data into chunks and generates embeddings to use for
vector search. See the [RAG component docs](https://convex.dev/components/rag)
for details, but here are some key features:

- **Namespaces:** Use namespaces for user-specific or team-specific data to
  isolate search domains.
- **Add Content**: Add or replace text content by key.
- **Semantic Search**: Vector-based search using configurable embedding models
- **Custom Filtering:** Define filters on each document for efficient vector
  search.
- **Chunk Context**: Get surrounding chunks for better context.
- **Importance Weighting**: Weight content by providing a 0 to 1 "importance" to
  affect per-document vector search results.
- **Chunking flexibility:** Bring your own document chunking, or use the
  default.
- **Graceful Migrations**: Migrate content or whole namespaces without
  disruption.

import { ComponentCardList } from "@site/src/components/ComponentCard";

<ComponentCardList
  items={[
    {
      title: "Install the RAG Component",
      description: "Get started with Retrieval-Augmented Generation.",
      href: "https://www.convex.dev/components/rag",
    },
  ]}
/>

## RAG Approaches

This directory contains two different approaches to implementing RAG:

### 1. Prompt-based RAG

A straightforward implementation where the system automatically searches for
relevant context for a user query.

- The message history will only include the original user prompt and the
  response, not the context.
- Looks up the context and injects it into the user's prompt.
- Works well if you know the user's question will _always_ benefit from extra
  context.

For example code, see
[ragAsPrompt.ts](https://github.com/get-convex/agent/blob/main/example/convex/rag/ragAsPrompt.ts)
for the overall code. The simplest version is:

```ts
const { thread } = await agent.continueThread(ctx, { threadId });
const context = await rag.search(ctx, {
  namespace: "global",
  query: userPrompt,
  limit: 10,
});

const result = await thread.generateText({
  prompt: `# Context:\n\n ${context.text}\n\n---\n\n# Question:\n\n"""${userPrompt}\n"""`,
});
```

### 2. Tool-based RAG

The LLM can intelligently decide when to search for context or add new
information by providing a tool to search for context.

- The message history will include the original user prompt and message history.
- After a tool call and response, the message history will include the tool call
  and response for the LLM to reference.
- The LLM can decide when to search for context or add new information.
- This works well if you want the Agent to be able to dynamically search.

See
[ragAsTools.ts](https://github.com/get-convex/agent/blob/main/example/convex/rag/ragAsTools.ts)
for the code. The simplest version is:

```ts
searchContext: createTool({
  description: "Search for context related to this user prompt",
  args: z.object({ query: z.string().describe("Describe the context you're looking for") }),
  handler: async (ctx, { query }) => {
    const context = await rag.search(ctx, { namespace: userId, query });
    return context.text;
  },
}),
```

## Key Differences

| Feature            | Basic RAG                    | Tool-based RAG                         |
| ------------------ | ---------------------------- | -------------------------------------- |
| **Context Search** | Always searches              | AI decides when to search              |
| **Adding Context** | Manual via separate function | AI can add context during conversation |
| **Flexibility**    | Simple, predictable          | Intelligent, adaptive                  |
| **Use Case**       | FAQ systems, document search | Dynamic knowledge management           |
| **Predictability** | Defined by code              | AI may query too much or little        |

## Examples in Action

To see these examples in action, check out the demo UI which provides:

- Context management UI
- Browse the chunks of documents in the RAG component
- Interactive chat interface
- Search result visualization
- Real-time streaming responses

Run the example with:

```bash
git clone https://github.com/get-convex/agent.git
cd agent
npm run setup
npm run example
```
