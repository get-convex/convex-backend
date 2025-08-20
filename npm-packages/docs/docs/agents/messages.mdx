---
title: Messages
sidebar_label: "Messages"
sidebar_position: 300
description: "Sending and receiving messages with an agent"
---

The Agent component stores message and [thread](./threads.mdx) history to enable
conversations between humans and agents.

To see how humans can act as agents, see [Human Agents](./human-agents.mdx).

## Generating a message

To generate a message, you provide a prompt (as a string or a list of messages)
to be used as context to generate one or more messages via an LLM, using calls
like `streamText` or `generateObject`.

The message history will be provided by default as context. See
[LLM Context](./context.mdx) for details on configuring the context provided.

The arguments to `generateText` and others are the same as the AI SDK, except
you don't have to provide a model. By default it will use the agent's chat
model.

Note: `authorizeThreadAccess` referenced below is a function you would write to
authenticate and authorize the user to access the thread. You can see an example
implementation in
[threads.ts](https://github.com/get-convex/agent/blob/main/example/convex/threads.ts).

See
[chat/basic.ts](https://github.com/get-convex/agent/blob/main/example/convex/chat/basic.ts)
or
[chat/streaming.ts](https://github.com/get-convex/agent/blob/main/example/convex/chat/streaming.ts)
for live code examples.

### Basic approach (synchronous)

```ts
export const generateReplyToPrompt = action({
  args: { prompt: v.string(), threadId: v.string() },
  handler: async (ctx, { prompt, threadId }) => {
    // await authorizeThreadAccess(ctx, threadId);
    const result = await agent.generateText(ctx, { threadId }, { prompt });
    return result.text;
  },
});
```

Note: best practice is to not rely on returning data from the action.Instead,
query for the thread messages via the `useThreadMessages` hook and receive the
new message automatically. See below.

### Saving the prompt then generating response(s) asynchronously

While the above approach is simple, generating responses asynchronously provide
a few benefits:

- You can set up optimistic UI updates on mutations that are transactional, so
  the message will be shown optimistically on the client until the message is
  saved and present in your message query.
- You can save the message in the same mutation (transaction) as other writes to
  the database. This message can the be used and re-used in an action with
  retries, without duplicating the prompt message in the history. See
  [workflows](./workflows.mdx) for more details.
- Thanks to the transactional nature of mutations, the client can safely retry
  mutations for days until they run exactly once. Actions can transiently fail.

Any clients listing the messages will automatically get the new messages as they
are created asynchronously.

To generate responses asynchronously, you need to first save the message, then
pass the `messageId` as `promptMessageId` to generate / stream text.

```ts
import { components, internal } from "./_generated/api";
import { saveMessage } from "@convex-dev/agent";
import { internalAction, mutation } from "./_generated/server";
import { v } from "convex/values";

// Step 1: Save a user message, and kick off an async response.
export const sendMessage = mutation({
  args: { threadId: v.id("threads"), prompt: v.string() },
  handler: async (ctx, { threadId, prompt }) => {
    const userId = await getUserId(ctx);
    const { messageId } = await saveMessage(ctx, components.agent, {
      threadId,
      userId,
      prompt,
      skipEmbeddings: true,
    });
    await ctx.scheduler.runAfter(0, internal.example.generateResponseAsync, {
      threadId,
      promptMessageId: messageId,
    });
  },
});

// Step 2: Generate a response to a user message.
export const generateResponseAsync = internalAction({
  args: { threadId: v.string(), promptMessageId: v.string() },
  handler: async (ctx, { threadId, promptMessageId }) => {
    await agent.generateText(ctx, { threadId }, { promptMessageId });
  },
});

// This is a common enough need that there's a utility to save you some typing.
// Equivalent to the above.
export const generateResponseAsync = agent.asTextAction();
```

Note: when calling `agent.saveMessage`, embeddings are generated automatically
when you save messages from an action and you have a text embedding model set.
However, if you're saving messages in a mutation, where calling an LLM is not
possible, it will generate them automatically when `generateText` receives a
`promptMessageId` that lacks an embedding and you have a text embedding model
configured. This is useful for workflows where you want to save messages in a
mutation, but not generate them. In these cases, pass `skipEmbeddings: true` to
`agent.saveMessage` to avoid the warning. If you're calling `saveMessage`
directly, you need to provide the embedding yourself, so `skipEmbeddings` is not
a parameter.

### Streaming

Streaming follows the same pattern as the basic approach, but with a few
differences, depending on the type of streaming you're doing.

The easiest way to stream is to pass `{ saveStreamDeltas: true }` to
`streamText`. This will save chunks of the response as deltas as they're
generated, so all clients can subscribe to the stream and get live-updating text
via normal Convex queries. See below for details on how to retrieve and display
the stream.

```ts
await foo.streamText(ctx, { threadId }, { prompt }, { saveStreamDeltas: true });
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

You can also consume the stream in all the ways you can with the underlying AI
SDK - for instance iterating over the content, or using
[`result.toDataStreamResponse()`](https://ai-sdk.dev/docs/reference/ai-sdk-core/stream-text#to-data-stream-response).

```ts
const result = await thread.streamText({ prompt });
// Note: if you do this, don't also call `.consumeStream()`.
for await (const textPart of result.textStream) {
  console.log(textPart);
}
```

### Saving deltas and returning an interactive stream

If you want to do both: iterate as the stream is happening, as well as save the
deltas, you can pass `{ saveStreamDeltas: { returnImmediately: true } }` to
`streamText`. This will return immediately, and you can then iterate over the
stream as it happens.

```ts
const result = await agent.streamText(
  ctx,
  { threadId },
  { prompt },
  { saveStreamDeltas: { returnImmediately: true } },
);

return result.toUIMessageStreamResponse();
```

See below for how to retrieve the stream deltas to a client.

### Generating an object

Similar to the AI SDK, you can generate or stream an object. The same arguments
apply, except you don't have to provide a model. It will use the agent's default
chat model.

```ts
import { z } from "zod/v3";

const result = await thread.generateObject({
  prompt: "Generate a plan based on the conversation so far",
  schema: z.object({...}),
});
```

## Retrieving messages

For streaming, it will save deltas to the database, so all clients querying for
messages will get the stream.

See
[chat/basic.ts](https://github.com/get-convex/agent/blob/main/example/convex/chat/basic.ts)
for the server-side code, and
[chat/streaming.ts](https://github.com/get-convex/agent/blob/main/example/convex/chat/streaming.ts)
for the streaming example.

You have a function that both allows paginating over messages. To support
streaming, you can also take in a `streamArgs` object and return the `streams`
result from `syncStreams`.

```ts
import { paginationOptsValidator } from "convex/server";
import { v } from "convex/values";
import { listMessages } from "@convex-dev/agent";
import { components } from "./_generated/api";

export const listThreadMessages = query({
  args: { threadId: v.string(), paginationOpts: paginationOptsValidator },
  handler: async (ctx, args) => {
    await authorizeThreadAccess(ctx, threadId);

    const paginated = await listMessages(ctx, components.agent, args);

    // Here you could filter out / modify the documents
    return paginated;
  },
});
```

### Retrieving streamed deltas

To retrieve the stream deltas, you only have to make a few changes to the query:

```ts
import { paginationOptsValidator } from "convex/server";
// highlight-next-line
import { vStreamArgs, listMessages, syncStreams } from "@convex-dev/agent";
import { components } from "./_generated/api";

export const listThreadMessages = query({
  args: {
    threadId: v.string(),
    paginationOpts: paginationOptsValidator,
    // highlight-next-line
    streamArgs: vStreamArgs,
  },
  handler: async (ctx, args) => {
    await authorizeThreadAccess(ctx, threadId);

    const paginated = await listMessages(ctx, components.agent, args);

    // highlight-next-line
    const streams = await syncStreams(ctx, components.agent, args);

    // highlight-next-line
    return { ...paginated, streams };
  },
});
```

You can then use the instructions below along with the `useSmoothText` hook to
show the streaming text in a UI.

## Showing messages in React

See
[ChatStreaming.tsx](https://github.com/get-convex/agent/blob/main/example/ui/chat/ChatStreaming.tsx)
for a streaming example, or
[ChatBasic.tsx](https://github.com/get-convex/agent/blob/main/example/ui/chat/ChatBasic.tsx)
for a non-streaming example.

### `useThreadMessages` hook

The crux is to use the `useThreadMessages` hook. For streaming, pass in
`stream: true` to the hook.

```tsx
import { api } from "../convex/_generated/api";
import { useThreadMessages, toUIMessages } from "@convex-dev/agent/react";

function MyComponent({ threadId }: { threadId: string }) {
  const messages = useThreadMessages(
    api.chat.streaming.listMessages,
    { threadId },
    { initialNumItems: 10, stream: true },
  );
  return (
    <div>
      {toUIMessages(messages.results ?? []).map((message) => (
        <div key={message.key}>{message.text}</div>
      ))}
    </div>
  );
}
```

### `toUIMessages` helper

```ts
import { toUIMessages, type UIMessage } from "@convex-dev/agent/react";
```

`toUIMessages` is a helper function that transforms messages into AI SDK
"UIMessage"s. This is a convenient data model for displaying messages:

- `parts` is an array of parts (e.g. "text", "file", "image", "toolCall",
  "toolResult")
- `content` is a string of the message content.
- `role` is the role of the message (e.g. "user", "assistant", "system").

The helper also adds some additional fields:

- `key` is a unique identifier for the message.
- `order` is the order of the message in the thread.
- `stepOrder` is the step order of the message in the thread.
- `status` is the status of the message (or "streaming").
- `agentName` is the name of the agent that generated the message.

To reference these, ensure you're importing `UIMessage` from
`@convex-dev/agent/react`.

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

### Optimistic updates for sending messages

The `optimisticallySendMessage` function is a helper function for sending a
message, so you can optimistically show a message in the message list until the
mutation has completed on the server.

Pass in the query that you're using to list messages, and it will insert the
ephemeral message at the top of the list.

```ts
const sendMessage = useMutation(
  api.streaming.streamStoryAsynchronously,
).withOptimisticUpdate(
  optimisticallySendMessage(api.streaming.listThreadMessages),
);
```

If your arguments don't include `{ threadId, prompt }` then you can use it as a
helper function in your optimistic update:

```ts
import { optimisticallySendMessage } from "@convex-dev/agent/react";

const sendMessage = useMutation(
  api.chatStreaming.streamStoryAsynchronously,
).withOptimisticUpdate(
  (store, args) => {
    optimisticallySendMessage(api.chatStreaming.listThreadMessages)(store, {
      threadId:
      prompt: /* change your args into the user prompt. */,
    })
  }
);
```

## Saving messages manually

By default, the Agent will save messages to the database automatically when you
provide them as a prompt, as well as all generated messages.

You can save messages to the database manually using `saveMessage` or
`saveMessages`.

- You can pass a `prompt` or a full `message` (`ModelMessage` type)
- The `metadata` argument is optional and allows you to provide more details,
  such as `sources`, `reasoningDetails`, `usage`, `warnings`, `error`, etc.

### Without the Agent class:

Note: If you aren't using the Agent class with a text embedding model set, you
need to pass an `embedding` if you want to save it at the same time.

```ts
import { saveMessage } from "@convex-dev/agent";

const { messageId } = await saveMessage(ctx, components.agent, {
  threadId,
  userId,
  message: { role: "assistant", content: result },
  metadata: [{ reasoning, usage, ... }] // See MessageWithMetadata type
  agentName: "my-agent",
  embedding: { vector: [0.1, 0.2, ...], model: "text-embedding-3-small" },
});
```

```ts
import { saveMessages } from "@convex-dev/agent";

const { messages } = await saveMessages(ctx, components.agent, {
  threadId,
  userId,
  messages: [{ role, content }, ...],
  metadata: [{ reasoning, usage, ... }, ...] // See MessageWithMetadata type
  agentName: "my-agent",
  embeddings: { model: "text-embedding-3-small", vectors: [[0.1...], ...] },
});
```

### Using the Agent class:

```ts
const { messageId } = await agent.saveMessage(ctx, {
  threadId,
  userId,
  prompt,
  metadata,
});
```

```ts
const { messages } = await agent.saveMessages(ctx, {
  threadId, userId,
  messages: [{ role, content }],
  metadata: [{ reasoning, usage, ... }] // See MessageWithMetadata type
});
```

If you are saving the message in a mutation and you have a text embedding model
set, pass `skipEmbeddings: true`. The embeddings for the message will be
generated lazily if the message is used as a prompt. Or you can provide an
embedding upfront if it's available, or later explicitly generate them using
`agent.generateEmbeddings`.

## Configuring the storage of messages

Generally the defaults are fine, but if you want to pass in multiple messages
and have them all saved (vs. just the last one), or avoid saving any input or
output messages, you can pass in a `storageOptions` object, either to the Agent
constructor or per-message.

The use-case for passing in multiple messages but not saving them is if you want
to include some extra messages for context to the LLM, but only the last message
is the user's actual request. e.g.
`messages = [...messagesFromRag, messageFromUser]`. The default is to save the
prompt and all output messages.

```ts
const result = await thread.generateText({ messages }, {
  storageOptions: {
    saveMessages: "all" | "none" | "promptAndOutput";
  },
});
```

## Message ordering

Each message has `order` and `stepOrder` fields, which are incrementing integers
specific to a thread.

When `saveMessage` or `generateText` is called, the message is added to the
thread's next `order` with a `stepOrder` of 0.

As response message(s) are generated in response to that message, they are added
at the same `order` with the next `stepOrder`.

To associate a response message with a previous message, you can pass in the
`promptMessageId` to `generateText` and others.

Note: if the `promptMessageId` is not the latest message in the thread, the
context for the message generation will not include any messages following the
`promptMessageId`.

## Deleting messages

You can delete messages by their `_id` (returned from `saveMessage` or
`generateText`) or `order` / `stepOrder`.

By ID:

```ts
await agent.deleteMessage(ctx, { messageId });
// batch delete
await agent.deleteMessages(ctx, { messageIds });
```

By order (start is inclusive, end is exclusive):

```ts
// Delete all messages with the same order as a given message:
await agent.deleteMessageRange(ctx, {
  threadId,
  startOrder: message.order,
  endOrder: message.order + 1,
});
// Delete all messages with order 1 or 2.
await agent.deleteMessageRange(ctx, { threadId, startOrder: 1, endOrder: 3 });
// Delete all messages with order 1 and stepOrder 2-4
await agent.deleteMessageRange(ctx, {
  threadId,
  startOrder: 1,
  startStepOrder: 2,
  endOrder: 2,
  endStepOrder: 5,
});
```

## Other utilities:

```ts
import { ... } from "@convex-dev/agent";
```

- `serializeDataOrUrl` is a utility function that serializes an AI SDK
  `DataContent` or `URL` to a Convex-serializable format.
- `filterOutOrphanedToolMessages` is a utility function that filters out tool
  call messages that don't have a corresponding tool result message.
- `extractText` is a utility function that extracts text from a
  `ModelMessage`-like object.

### Validators and types

There are types to validate and provide types for various values

```ts
import { ... } from "@convex-dev/agent";
```

- `vMessage` is a validator for a `ModelMessage`-like object (with a `role` and
  `content` field e.g.).
- `MessageDoc` and `vMessageDoc` are the types for a message (which includes a
  `.message` field with the `vMessage` type).
- `Thread` is the type of a thread returned from `continueThread` or
  `createThread`.
- `ThreadDoc` and `vThreadDoc` are the types for thread metadata.
- `AgentComponent` is the type of the installed component (e.g.
  `components.agent`).
- `ToolCtx` is the `ctx` type for calls to `createTool` tools.
