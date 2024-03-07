// @snippet start list
import { v } from "convex/values";
import { query, mutation } from "./_generated/server";

export const list = query(async (ctx) => {
  return await ctx.db.query("messages").collect();
});
// @snippet end list

// @snippet start send
export const send = mutation({
  args: { body: v.string(), author: v.string() },
  handler: async (ctx, { body, author }) => {
    const message = { body, author };
    await ctx.db.insert("messages", message);
  },
});
// @snippet end send
