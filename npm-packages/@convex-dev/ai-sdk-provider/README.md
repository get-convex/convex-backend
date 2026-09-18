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

## Structured decisions

[Jev](https://docs.typesafe.ai/introduction) evaluates `choice`, `score`, and
`noul` questions about the context you provide in `state`. It returns structured
answers your code can use directly. Call `convexGateway.decisions()` from an
action:

```ts
const decision = await convexGateway.decisions({
  model: "typesafe/jev-1.13",
  state: { ticket: "Customer cannot sign in" },
  questions: {
    priority: {
      type: "choice",
      instructions: "Choose the response priority",
      criteria: {
        urgent: "Respond now",
        normal: "Respond today",
      },
    },
    needsReview: {
      type: "noul",
      instructions: "Does a human need to review this?",
    },
  },
});

console.log(decision.answers.priority.choice);
console.log(decision.answers.needsReview.noul);
```

The method returns answers rather than a model for `generateText`.
Authentication is handled automatically. Pass `{ signal }` as the second
argument to cancel a request with an `AbortSignal`. HTTP errors and invalid
responses throw `ConvexGatewayError`.

`getServiceToken("ai-gateway")` mints a short-lived deployment JWT on first use
in an action and reuses it for later calls, so `convexGateway(...)` is fine to
call more than once. The provider takes no API key.

Requires Convex 1.45 or later, AI SDK 7, and Node.js 22 or later.

## Generate images

```ts
import { generateImage } from "ai";
import { convexGateway } from "@convex-dev/ai-sdk-provider";

const { images } = await generateImage({
  model: convexGateway.imageModel("openai/gpt-image-1"),
  prompt: "A mountain lake at sunrise",
});
```

## Generate videos

```ts
import { experimental_generateVideo as generateVideo } from "ai";
import { convexGateway } from "@convex-dev/ai-sdk-provider";

const { video, providerMetadata } = await generateVideo({
  model: convexGateway.videoModel("google/veo-3.1"),
  prompt: "A camera pan across a mountain lake",
  duration: 8,
  aspectRatio: "16:9",
  resolution: "1280x720",
});

const bytes = video.uint8Array;
const cost = providerMetadata?.convexGateway?.cost;
```

The call waits for generation and download, with a ten-minute default timeout
and a 64 MiB limit per video. Store the returned bytes in Convex file storage.
The AI SDK splits `n > 1` into separate requests. Cancelling a request does not
cancel the upstream job and can still incur a charge.

An image in the prompt becomes the first frame. Use `frameImages` for explicit
frames, `inputReferences` for image/audio/video references, and `generateAudio`
for audio. `fps` returns an unsupported-option warning.

`providerOptions.convexGateway` accepts `resolution` (such as `720p`),
`generate_audio`, `frame_images`, and `input_references`. Standard SDK options
take precedence. Supported values depend on the
[OpenRouter model](https://openrouter.ai/docs/guides/overview/multimodal/video-generation).
