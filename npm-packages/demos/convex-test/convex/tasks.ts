import { mutation, query } from "./_generated/server";
import { v } from "convex/values";

export const create = mutation({
  args: {
    text: v.string(),
  },
  handler: async (ctx, args) => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity) {
      throw new Error("Not authenticated");
    }

    await ctx.db.insert("tasks", {
      text: args.text,
      author: identity.name ?? "Unknown",
    });
  },
});

export const list = query({
  args: {},
  handler: async (ctx) => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity) {
      throw new Error("Not authenticated");
    }

    const tasks = await ctx.db
      .query("tasks")
      .withIndex("by_author", (q) => q.eq("author", identity.name))
      .collect();

    return tasks;
  },
});
