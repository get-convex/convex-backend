import { v } from "convex/values";
import { mutation } from "./_generated/server";

// @snippet start send
// highlight-start
import { api } from "./_generated/api";
// highlight-end

// ...

export const send = mutation({
  args: { body: v.string(), author: v.string() },
  handler: async (ctx, args) => {
    const { body, author } = args;
    // Send a new message.
    await ctx.db.insert("messages", { body, author });

    // highlight-start
    if (body.startsWith("@ai") && author !== "AI") {
      // Schedule the chat action to run immediately
      await ctx.scheduler.runAfter(0, api.ai.chat, {
        messageBody: body,
      });
    }
    // highlight-end
  },
});
// @snippet end send
