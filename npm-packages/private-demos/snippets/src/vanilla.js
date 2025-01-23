import { ConvexClient } from "convex/browser";
import { api } from "../convex/_generated/api";

const client = new ConvexClient(process.env.CONVEX_URL);

// subscribe to query results
client.onUpdate(api.messages.listAll, {}, (messages) =>
  console.log(messages.map((msg) => msg.body)),
);

// execute a mutation
function hello() {
  client.mutation(api.messages.sendAnon, {
    body: `hello at ${new Date()}`,
  });
}
