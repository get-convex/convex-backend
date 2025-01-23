import { v } from "convex/values";
import { mutation } from "./_generated/server";
import { getCurrentUserIdOrThrow } from "./users";

export const create = mutation({
  args: {
    emoji: v.string(),
  },
  handler: async (ctx, args) => {
    const userId = await getCurrentUserIdOrThrow(ctx);
    const id = await ctx.db.insert("conversations", {
      emoji: args.emoji,
    });
    await ctx.db.insert("conversationMembers", {
      conversationId: id,
      userId,
      hasUnreadMessages: false,
      latestMessageTime: Date.now(),
    });
    return id;
  },
});
