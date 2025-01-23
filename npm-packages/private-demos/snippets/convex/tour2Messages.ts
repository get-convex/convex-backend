import { mutation, query } from "./_generated/server";
import { v } from "convex/values";

// @snippet start likeTodo
export const like = mutation({
  args: { liker: v.string(), messageId: v.id("messages") },
  handler: async (ctx, args) => {
    // TODO
  },
});
// @snippet end likeTodo

export const list_ = query({
  args: {},
  handler: async (ctx) => {
    // Grab the most recent messages.
    const messages = await ctx.db.query("messages").order("desc").take(100);
    const messagesWithLikes = await Promise.all(
      messages.map(async (message) => {
        // Find the likes for each message
        // @snippet start likesWithIndex
        const likes = await ctx.db
          .query("likes")
          .withIndex("byMessageId", (q) => q.eq("messageId", message._id))
          .collect();
        // @snippet end likesWithIndex
        // Join the count of likes with the message data
        return {
          ...message,
          likes: likes.length,
        };
      }),
    );
    const messagesWithLikes2 = await Promise.all(
      messages.map(async (message) => {
        // Find the likes for each message
        // @snippet start likesWithFilter
        const likes = await ctx.db
          .query("likes")
          .filter((q) => q.eq(q.field("messageId"), message._id))
          .collect();
        // @snippet end likesWithFilter
        // Join the count of likes with the message data
        return {
          ...message,
          likes: likes.length,
        };
      }),
    );
    // Reverse the list so that it's in chronological order.
    return messagesWithLikes.reverse();
  },
});
