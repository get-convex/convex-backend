import { query } from "./_generated/server";

// List all chat messages in the given channel.
export default query(async function listMessages(
  { db },
  { channel }: { channel: string },
) {
  return await db
    .query("messages")
    .withIndex("by_channel", (q) => q.eq("channel", channel))
    .collect();
});
