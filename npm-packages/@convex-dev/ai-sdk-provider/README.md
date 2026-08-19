# `@convex-dev/ai-sdk-provider`

Use the Convex AI gateway with the AI SDK from a Convex action.

```ts
"use node";

import { convexGateway } from "@convex-dev/ai-sdk-provider";
import { generateText } from "ai";
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
```

`getServiceToken("ai-gateway")` mints a short-lived deployment JWT on first use
in an action and reuses it for later calls, so `convexGateway(...)` is fine to
call more than once. The provider takes no API key.

Requires Convex 1.44 or later, AI SDK 7, and Node.js 22 or later.
