---
title: Files and Images in Agent messages
sidebar_label: "Files"
sidebar_position: 1000
description: "Working with images and files in the Agent component"
---

You can add images and files for the LLM to reference in the messages.

NOTE: Sending URLs to LLMs is much easier with the cloud backend, since it has
publicly available storage URLs. To develop locally you can use `ngrok` or
similar to proxy the traffic.

Example code:

- [files/autoSave.ts](https://github.com/get-convex/agent/blob/main/example/convex/files/autoSave.ts)
  has a simple example of how to use the automatic file saving.
- [files/addFile.ts](https://github.com/get-convex/agent/blob/main/example/convex/files/addFile.ts)
  has an example of how to save the file, submit a question, and generate a
  response in separate steps.
- [files/generateImage.ts](https://github.com/get-convex/agent/blob/main/example/convex/files/generateImage.ts)
  has an example of how to generate an image and save it in an assistant
  message.
- [FilesImages.tsx](https://github.com/get-convex/agent/blob/main/example/ui/files/FilesImages.tsx)
  has client-side code.

## Running the example

```sh
git clone https://github.com/get-convex/agent.git
cd agent
npm run setup
npm run example
```

## Sending an image by uploading first and generating asynchronously

The standard approach is to:

1. Upload the file to the database (`uploadFile` action). Note: this can be in a
   regular action or in an httpAction, depending on what's more convenient.
2. Send a message to the thread (`submitFileQuestion` action)
3. Send the file to the LLM to generate / stream text asynchronously
   (`generateResponse` action)
4. Query for the messages from the thread (`listThreadMessages` query)

Rationale:

It's better to submit a message in a mutation vs. an action because you can use
an optimistic update on the client side to show the sent message immediately and
have it disappear exactly when the message comes down in the query.

However, you can't save to file storage from a mutation, so the file needs to
already exist (hence the fileId).

You can then asynchronously generate the response (with retries / etc) without
the client waiting.

### 1: Saving the file

```ts
import { storeFile } from "@convex-dev/agent";
import { components } from "./_generated/api";

const { file } = await storeFile(
  ctx,
  components.agent,
  new Blob([bytes], { type: mimeType }),
  {
    filename,
    sha256,
  },
);
const { fileId, url, storageId } = file;
```

### 2: Sending the message

```ts
// in your mutation
const { filePart, imagePart } = await getFile(ctx, components.agent, fileId);
const { messageId } = await fileAgent.saveMessage(ctx, {
  threadId,
  message: {
    role: "user",
    content: [
      imagePart ?? filePart, // if it's an image, prefer that kind.
      { type: "text", text: "What is this image?" },
    ],
  },
  metadata: { fileIds: [fileId] }, // IMPORTANT: this tracks the file usage.
});
```

### 3: Generating the response & querying the responses

This is done in the same way as text inputs.

```ts
// in an action
await thread.generateText({ promptMessageId: messageId });
```

```ts
// in a query
const messages = await agent.listMessages(ctx, { threadId, paginationOpts });
```

## Inline saving approach

You can also pass in an image / file direction when generating text, if you're
in an action. Any image or file passed in the `message` argument will
automatically be saved in file storage if it's larger than 64k, and a fileId
will be saved to the message.

Example:

```ts
await thread.generateText({
  message: {
    role: "user",
    content: [
      { type: "image", image: imageBytes, mimeType: "image/png" },
      { type: "text", text: "What is this image?" },
    ],
  },
});
```

## Under the hood

Saving to the files has 3 components:

1. Saving to file storage (in your app, not in the component's storage). This
   means you can access it directly with the `storageId` and generate URLs.
2. Saving a reference (the storageId) to the file in the component. This will
   automatically keep track of how many messages are referencing the file, so
   you can vacuum files that are no longer used (see
   [files/vacuum.ts](https://github.com/get-convex/agent/blob/main/example/convex/files/vacuum.ts)).
3. Inserting a URL in place of the data in the message sent to the LLM, along
   with the mimeType and other metadata provided. It will be inferred if not
   provided in
   [`guessMimeType`](https://github.com/get-convex/agent/blob/main/src/mapping.ts#L227).

### Can I just store the file myself an pass in a URL?

Yes! You can always pass a URL in the place of an image or file to the LLM.

```ts
const storageId = await ctx.storage.store(blob);
const url = await ctx.storage.getUrl(storageId);

await thread.generateText({
  message: {
    role: "user",
    content: [
      { type: "image", data: url, mimeType: blob.type },
      { type: "text", text: "What is this image?" },
    ],
  },
});
```

## Generating images

There's an example in
[files/generateImage.ts](https://github.com/get-convex/agent/blob/main/example/convex/files/generateImage.ts)
that takes a prompt, generates an image with OpenAI's dall-e 2, then saves the
image to a thread.

You can try it out with:

```sh
npx convex run files:generateImage:replyWithImage '{prompt: "make a picture of a cat" }'
```
