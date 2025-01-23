---
title: "Bun"
hidden: false
sidebar_position: 200
---

[Bun](https://bun.sh/) can be used to run scripts and servers that use Convex
clients and can even run the Convex CLI.

Convex supports point-in-time queries, mutations and actions (see
[HTTP client](/api/classes/browser.ConvexHttpClient)) and those plus query
subscriptions (see [ConvexClient](/api/classes/browser.ConvexClient)) in Bun.

```js
import { ConvexHttpClient, ConvexClient } from "convex/browser";
import { api } from "./convex/_generated/api.js";

// HTTP client
const httpClient = new ConvexHttpClient(process.env.CONVEX_URL);
httpClient.query(api.messages.list).then((messages) => {
  console.log(messages);
});

// Subscription client
const client = new ConvexClient(process.env.CONVEX_URL);
const unsubscribe = client.onUpdate(api.messages.list, {}, (messages) =>
  console.log(messages),
);
await Bun.sleep(1000);
client.mutate(api.messages.send, {}, { body: "hello!", author: "me" });
await Bun.sleep(1000);
```

## Using Convex with Bun without codegen

You can always use the `anyApi` object or strings if you don't have the Convex
functions and api file handy. An api reference like `api.folder.file.exportName`
becomes `anyApi.folder.file.exportName` or `"folder/file:exportName"`.
