import { v } from "convex/values";
import { mutation, query } from "./_generated/server";

export const list = query({
  args: {
    list: v.string(),
  },
  handler: async (ctx) => {
    return await ctx.db.query("tasks").collect();
  },
});

export const create = mutation({
  args: {
    text: v.string(),
  },
  handler: async (ctx, { text }) => {
    return await ctx.db.insert("tasks", { text });
  },
});
