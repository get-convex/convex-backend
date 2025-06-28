import { mutation } from "./_generated/server";

// Send a message to the given chat channel.
export default mutation({
  handler: async (
    { db },
    {
      format,
      body,
      author,
      extras,
    }: { format: "text" | "giphy"; body: string; author: string; extras?: any },
  ) => {
    console.log("running sendMessage mutation");
    const message = {
      body,
      author,
      format,
      extras,
    };
    await db.insert("messages", message);
  },
});

export const clearMessages = mutation({
  args: {},

  handler: async (ctx) => {
    for (const message of await ctx.db.query("messages").collect()) {
      await ctx.db.delete(message._id);
    }
  },
});
