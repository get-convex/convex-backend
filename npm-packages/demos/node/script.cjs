const { ConvexHttpClient, ConvexClient } = require("convex/browser");
const { api } = require("./convex/_generated/api_cjs.cjs");
require("dotenv").config({ path: ".env" });

async function main() {
  // Use the `ConvexHttpClient` to get the value of a query at a point in time
  // and run mutations and actions.
  const httpClient = new ConvexHttpClient(process.env["CONVEX_URL"]);

  const messages = await httpClient.query(api.messages.list);
  console.log(messages);

  // Use the `ConvexClient` to subscribe to queries as well as run mutations
  // and actions.
  const client = new ConvexClient(process.env["CONVEX_URL"]);
  const unsubscribe = client.onUpdate(api.messages.list, {}, (messages) => {
    console.log("\x1b[2J"); // clear the screen
    for (const message of messages.reverse()) {
      console.log(message.author, "says", message.body);
    }
    console.log();
    console.log("Last update at", new Date());
    console.log("ctrl-c to exit");
  });

  // this code runs on ctrl-c
  process.on("SIGINT", async () => {
    // It's not actually necessary to unsubscribe or close the WebSocket
    // connection when exiting the process, but it is important cleanup
    // when your program is going to keep running.
    unsubscribe();
    console.log("all done");
    await client.close();
    process.exit();
  });
}
main();
