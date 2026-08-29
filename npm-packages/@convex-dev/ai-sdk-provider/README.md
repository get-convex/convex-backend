# `@convex-dev/ai-sdk-provider`

Use the Convex AI gateway with the AI SDK from a Convex action.

```ts
"use node";

import { convexGateway } from "@convex-dev/ai-sdk-provider";
import { embedMany, generateText } from "ai";
import { action } from "./_generated/server";
import { v } from "convex/values";

export const chat = action({
  args: { prompt: v.string() },
  handler: async (_ctx, { prompt }) => {
    const { text } = await generateText({
      model: convexGateway("anthropic/claude-sonnet-4.5"),
      prompt,
    });
    return text;
  },
});

export const embed = action({
  args: { values: v.array(v.string()) },
  handler: async (_ctx, { values }) => {
    const { embeddings } = await embedMany({
      model: convexGateway.embeddingModel("openai/text-embedding-3-small"),
      values,
    });
    return embeddings;
  },
});
```

Embedding batches larger than the gateway's 512-input limit are split into
multiple requests by the AI SDK.

## Choose a model interface

For text generation, start with `convexGateway(model)`. It works across model
providers and is the recommended default. Use an endpoint-specific model only
when you need features unique to Anthropic Messages or OpenAI Responses:

```ts
convexGateway.messages("anthropic/claude-sonnet-4.5");
convexGateway.responses("openai/gpt-5");
```

`getServiceToken("ai-gateway")` mints a short-lived deployment JWT on first use
in an action and reuses it for later calls, so `convexGateway(...)` is fine to
call more than once. The provider takes no API key.

Requires Convex 1.45 or later, AI SDK 7, and Node.js 22 or later.
