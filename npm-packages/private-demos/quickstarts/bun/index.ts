import { ConvexClient } from "convex/browser";
import { api } from "./convex/_generated/api.js";

const client = new ConvexClient(process.env["CONVEX_URL"]);

const unsubscribe = client.onUpdate(api.tasks.get, {}, async (tasks) => {
  console.log(tasks);
});

await Bun.sleep(1000);
unsubscribe();
await client.close();
