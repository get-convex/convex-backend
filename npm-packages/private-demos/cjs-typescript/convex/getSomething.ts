import { Doc } from "./_generated/dataModel";
import { query } from "./_generated/server";

// List all chat messages in the given channel.
export default query(async ({ db }): Promise<string> => {
  const empty: Doc<"messages">[] = await db
    .query("messages")
    .order("desc")
    .take(50);
  const first: Doc<"messages"> = empty[0];
  return first ? first.body : "something";
});
