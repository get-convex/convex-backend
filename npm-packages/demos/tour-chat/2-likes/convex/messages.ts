import { query, mutation } from "./_generated/server";
import { v } from "convex/values";

// @snippet start listLikes
export const list = query({
  args: {},
  handler: async (ctx) => {
    // Grab the most recent messages.
    const messages = await ctx.db.query("messages").order("desc").take(100);
    const messagesWithLikes = await Promise.all(
      messages.map(async (message) => {
        // Find the likes for each message
        const likes = await ctx.db
          .query("likes")
          .withIndex("byMessageId", (q) => q.eq("messageId", message._id))
          .collect();
        // Join the count of likes with the message data
        return {
          ...message,
          likes: likes.length,
        };
      }),
    );
    // Reverse the list so that it's in a chronological order.
    return messagesWithLikes.reverse().map((message) => ({
      ...message,
      // Format smileys
      body: message.body.replaceAll(":)", "😊"),
    }));
  },
});
// @snippet end listLikes

export const send = mutation({
  args: { body: v.string(), author: v.string() },
  handler: async (ctx, args) => {
    // Send a new message.
    await ctx.db.insert("messages", { body: args.body, author: args.author });
  },
});

// @snippet start like
export const like = mutation({
  args: { liker: v.string(), messageId: v.id("messages") },
  handler: async (ctx, args) => {
    // Save a user's "like" of a particular message
    await ctx.db.insert("likes", {
      liker: args.liker,
      messageId: args.messageId,
    });
  },
});
// @snippet end like
