import { ConvexHttpClient } from "convex/browser";
import { api } from "../convex/_generated/api";

require("dotenv").config();

const client = new ConvexHttpClient(process.env.VITE_CONVEX_URL!);

/**
 * Logs an array containing all messages from the paginated query "listMessages"
 * by combining pages of results into a single array.
 */
async function getAllMessages() {
  let continueCursor = null;
  let isDone = false;
  let page;

  const results = [];

  while (!isDone) {
    ({ continueCursor, isDone, page } = await client.query(api.messages.list, {
      paginationOpts: { numItems: 5, cursor: continueCursor },
    }));
    console.log("got", page.length);
    results.push(...page);
  }

  console.log(results);
}

getAllMessages();
