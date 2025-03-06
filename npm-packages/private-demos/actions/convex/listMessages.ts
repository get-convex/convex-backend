import { Doc } from "./_generated/dataModel";
import { query } from "./_generated/server";

// List all chat messages in the given channel.
export default query({
  handler: async ({ db }): Promise<Doc<"messages">[]> => {
    return await db.query("messages").order("desc").take(50);
  },
});
