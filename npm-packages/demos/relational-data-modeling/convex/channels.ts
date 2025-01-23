import { v } from "convex/values";
import { query, mutation } from "./_generated/server";

export const list = query(async (ctx) => {
  return await ctx.db.query("channels").collect();
});

export const add = mutation({
  args: { name: v.string() },
  handler: async (ctx, { name }) => {
    return ctx.db.insert("channels", { name });
  },
});
