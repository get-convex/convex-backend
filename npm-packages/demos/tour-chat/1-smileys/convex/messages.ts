import { query, mutation } from "./_generated/server";
import { v } from "convex/values";

// @snippet start messages:list
export const list = query({
  args: {},
  handler: async (ctx) => {
    // Grab the most recent messages.
    const messages = await ctx.db.query("messages").order("desc").take(100);
    // Reverse the list so that it's in a chronological order.
    return messages.reverse().map((message) => ({
      ...message,
      // Format smileys
      body: message.body.replaceAll(":)", "😊"),
    }));
  },
});
// @snippet end messages:list

// @snippet start messages:send
export const send = mutation({
  args: { body: v.string(), author: v.string() },
  handler: async (ctx, { body, author }) => {
    // Send a new message.
    await ctx.db.insert("messages", { body, author });
  },
});
// @snippet end messages:send
