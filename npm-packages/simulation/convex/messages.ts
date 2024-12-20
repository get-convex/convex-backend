import { v } from "convex/values";
import { mutation } from "./_generated/server";
import { getCurrentUserIdOrThrow } from "./users";

export const send = mutation({
  args: {
    body: v.string(),
    conversationId: v.id("conversations"),

    author: v.id("users"),
    id: v.string(),
    creationTime: v.number(),
  },
  handler: async (ctx, args) => {
    const author = await getCurrentUserIdOrThrow(ctx);
    if (args.author !== author) {
      throw new Error("User does not match");
    }
    // Send a new message.
    await ctx.db.insert("messages", {
      body: args.body,
      author,
      conversationId: args.conversationId,
    });
  },
});
