---
title: Streaming
sidebar_label: "Streaming"
sidebar_position: 340
description: "Streaming messages with an agent"
---

Streaming messages is a great way to give a user feedback and keep an
application feeling responsive while using LLMs.

Traditionally streaming happens via HTTP streaming, where the client sends a
request and waits until the full response is streamed back. This works out of
the box when using the Agent, in the same way you would with the AI SDK. See
[below](#consuming-the-stream-yourself-with-the-agent) if that is all you're
looking for.

However, with the Agent component you can also stream messages asynchronously,
meaning the generation doesn't have to happen in an HTTP handler (`httpAction`),
and the response can be streamed back to one or more clients even if their
network connection is interrupted.

It works by saving the streaming parts to the database in groups (deltas), and
the clients subscribe to new deltas for the given thread, as they're generated.
As a bonus, you don't even need to use the Agent's version of `streamText` to
use the delta streaming approach (see
[below](#advanced-streaming-deltas-asynchronously-without-using-an-agent)).

Example:

- Server:
  [streaming.ts](https://github.com/get-convex/agent/blob/main/example/convex/chat/streaming.ts)
- Client:
  [ChatStreaming.tsx](https://github.com/get-convex/agent/blob/main/example/ui/chat/ChatStreaming.tsx)

## Streaming message deltas

The easiest way to stream is to pass `{ saveStreamDeltas: true }` to
`agent.streamText`. This will save chunks of the response as deltas as they're
generated, so all clients can subscribe to the stream and get live-updating text
via normal Convex queries.

```ts
agent.streamText(ctx, { threadId }, { prompt }, { saveStreamDeltas: true });
```

This can be done in an async function, where http streaming to a client is not
possible. Under the hood it will chunk up the response and debounce saving the
deltas to prevent excessive bandwidth usage. You can pass more options to
`saveStreamDeltas` to configure the chunking and debouncing.

```ts
  { saveStreamDeltas: { chunking: "line", throttleMs: 1000 } },
```

- `chunking` can be "word", "line", a regex, or a custom function.
- `throttleMs` is how frequently the deltas are saved. This will send multiple
  chunks per delta, writes sequentially, and will not write faster than the
  throttleMs
  ([single-flighted](https://stack.convex.dev/throttling-requests-by-single-flighting)
  ).

## Retrieving streamed deltas

For clients to stream messages, you need to expose a query that returns the
stream deltas. This is very similar to
[retrieving messages](./messages.mdx#retrieving-messages), with a few changes:

```ts
import { paginationOptsValidator } from "convex/server";
// highlight-next-line
import { vStreamArgs, listUIMessages, syncStreams } from "@convex-dev/agent";
import { components } from "./_generated/api";

export const listThreadMessages = query({
  args: {
    threadId: v.string(),
    // Pagination options for the non-streaming messages.
    paginationOpts: paginationOptsValidator,
    // highlight-next-line
    streamArgs: vStreamArgs,
  },
  handler: async (ctx, args) => {
    await authorizeThreadAccess(ctx, threadId);

    // Fetches the regular non-streaming messages.
    const paginated = await listUIMessages(ctx, components.agent, args);

    // highlight-next-line
    const streams = await syncStreams(ctx, components.agent, args);

    // highlight-next-line
    return { ...paginated, streams };
  },
});
```

Similar to with [non-streaming messages](./messages.mdx#useuimessages-hook), you
can use the `useUIMessages` hook to fetch the messages, passing in
`stream: true` to enable streaming.

```ts
const { results, status, loadMore } = useUIMessages(
  api.chat.streaming.listMessages,
  { threadId },
  { initialNumItems: 10, stream: true },
);
```

### Text smoothing with `SmoothText` and `useSmoothText`

The `useSmoothText` hook is a simple hook that smooths the text as it changes.
It can work with any text, but is especially handy for streaming text.

```ts
import { useSmoothText } from "@convex-dev/agent/react";

// in the component
const [visibleText] = useSmoothText(message.text);
```

You can configure the initial characters per second. It will adapt over time to
match the average speed of the text coming in.

By default it won't stream the first text it receives unless you pass in
`startStreaming: true`. To start streaming immediately when you have a mix of
streaming and non-streaming messages, do:

```ts
import { useSmoothText, type UIMessage } from "@convex-dev/agent/react";

function Message({ message }: { message: UIMessage }) {
  const [visibleText] = useSmoothText(message.text, {
    startStreaming: message.status === "streaming",
  });
  return <div>{visibleText}</div>;
}
```

If you don't want to use the hook, you can use the `SmoothText` component.

```tsx
import { SmoothText } from "@convex-dev/agent/react";

//...
<SmoothText text={message.text} />;
```

## Consuming the stream yourself with the Agent

You can consume the stream in all the ways you can with the underlying AI SDK -
for instance iterating over the content, or using
[`result.toDataStreamResponse()`](https://ai-sdk.dev/docs/reference/ai-sdk-core/stream-text#to-data-stream-response).

If you are not also saving the deltas, it might look like this:

```ts
const result = await agent.streamText(ctx, { threadId }, { prompt });

for await (const textPart of result.textStream) {
  console.log(textPart);
}
```

If you want to both iterate as the stream is happening, as well as save the
deltas, you can pass `{ saveStreamDeltas: { returnImmediately: true } }` to
`streamText`. This will return immediately, and you can then iterate over the
stream live, or return the stream in an HTTP Response.

```ts
const result = await agent.streamText(
  ctx,
  { threadId },
  { prompt },
  { saveStreamDeltas: { returnImmediately: true } },
);

return result.toUIMessageStreamResponse();
```

If you don't want to have the Agent involved at all, the next section will show
you how to save the deltas yourself.

## Advanced: Streaming deltas asynchronously without using an Agent

To stream messages without using the Agent's wrapper of `streamText`, you can
use the `streamText` function from the AI SDK directly.

It consists of using the `DeltaStreamer` class to save the deltas to the
database, and then using the above approach to retrieve the messages, though you
can use a more direct `useStreamingUIMessages` hook that doesn't involve reading
any non-streaming messages from the database.

The requirements for reading and writing the streams are just that they use a
`threadId` from the Agent component, and that each stream is saved with a
distinct `order`, for ordering on the client side.

```ts
import { components } from "./_generated/api";
import { type ActionCtx } from "./_generated/server";
import { DeltaStreamer, compressUIMessageChunks } from "@convex-dev/agent";
import { streamText } from "ai";
import { openai } from "@ai-sdk/openai";

async function stream(ctx: ActionCtx, threadId: string, order: number) {
  const streamer = new DeltaStreamer(
    components.agent,
    ctx,
    {
      throttleMs: 100,
      onAsyncAbort: async () => console.error("Aborted asynchronously"),
      // This will collapse multiple tiny deltas into one if they're being sent
      // in quick succession.
      compress: compressUIMessageChunks,
      abortSignal: undefined,
    },
    {
      threadId,
      format: "UIMessageChunk",
      order,
      stepOrder: 0,
      userId: undefined,
    },
  );
  // Do the normal streaming with the AI SDK
  const response = streamText({
    model: openai.chat("gpt-4o-mini"),
    prompt: "Tell me a joke",
    abortSignal: streamer.abortController.signal,
    onError: (error) => {
      console.error(error);
      streamer.fail(errorToString(error.error));
    },
  });

  // We could await here if we wanted to wait for the stream to finish,
  // but instead we have it process asynchronously so we can return a streaming
  // http Response.
  void streamer.consumeStream(response.toUIMessageStream());

  return {
    // e.g. to do `response.toTextStreamResponse()` for HTTP streaming.
    response,
    // We don't need this on the client, but with it we can have some clients
    // selectively not stream down deltas when they're using HTTP streaming
    // already.
    streamId: await streamer.getStreamId(),
  };
}
```

To fetch the deltas for the client, you can use the `syncStreams` function, as
you would with normal Agent streaming. If you don't want to fetch the
non-streaming messages, it can be simplified to:

```ts
import { v } from "convex/values";
import { vStreamArgs, syncStreams } from "@convex-dev/agent";
import { query } from "./_generated/server";
import { components } from "./_generated/api";

export const listStreams = query({
  args: {
    threadId: v.string(),
    streamArgs: vStreamArgs,
  },
  handler: async (ctx, args) => {
    // await authorizeThreadAccess(ctx, args.threadId);
    const streams = await syncStreams(ctx, components.agent, {
      ...args,
      // By default syncStreams only returns streaming messages. However, if
      // your messages aren't saved in the same transaction as the streaming
      // ends, you might want to include them here to avoid UI flashes.
      includeStatuses: ["streaming", "aborted", "finished"],
    });
    return { streams };
  },
});
```

On the client side, you can use the `useStreamingUIMessages` hook to fetch the
messages. If you defined more arguments than just `threadId`, they'll get passed
along with `threadId` here.

```ts
const messages = useStreamingUIMessages(api.example.listStreams, { threadId });
```

You can pass in another parameter to either skip certain `streamId`s or to start
at some `order` to ignore previous streams.
