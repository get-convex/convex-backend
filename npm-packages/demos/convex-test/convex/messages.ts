import { ConvexError, v } from "convex/values";
import { action, mutation } from "./_generated/server";

export const sendAIMessage = action({
  args: { prompt: v.string() },
  handler: async (): Promise<string> => {
    const response = await fetch("https://www.example.com/ai");
    const text = await response.text();
    return text;
  },
});

export const send = mutation({
  args: { body: v.string(), author: v.string() },
  handler: async (ctx, { body, author }) => {
    if (body === "") {
      throw new ConvexError("Empty message body is not allowed");
    }

    await ctx.db.insert("messages", { body, author });
  },
});
