import { v } from "convex/values";
import { mutation } from "./_generated/server";

// @snippet start send
export const send = mutation({
  args: { body: v.string(), author: v.string() },
  returns: v.null(),
  handler: async (ctx, { body, author }) => {
    const message = { body, author };
    await ctx.db.insert("messages", message);
  },
});
// @snippet end send
