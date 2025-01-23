import { ConvexClient } from "convex/browser";
import { anyApi } from "convex/server";

const CONVEX_URL = "http://happy-otter-123";
const client = new ConvexClient(CONVEX_URL);
client.onUpdate(anyApi.messages.list, {}, (messages) =>
  console.log(messages.map((msg) => msg.body)),
);
setInterval(
  () =>
    client.mutation(anyApi.messages.send, {
      body: `hello at ${new Date()}`,
      author: "me",
    }),
  5000,
);
