import { v } from "convex/values";
import { api, internal } from "./_generated/api";
import { action, internalMutation, mutation, query } from "./_generated/server";

export const load = query({
  handler: async (ctx) => {
    const doc = await ctx.db.query("counter").first();
    return doc?.count ?? 0;
  },
});

export const increment = mutation({
  handler: async (ctx) => {
    const doc = await ctx.db.query("counter").first();
    if (!doc) {
      await ctx.db.insert("counter", { count: 1 });
      return;
    }
    await ctx.db.patch(doc._id, { count: doc.count + 1 });
  },
});

export const reset = action({
  args: {
    count: v.number(),
  },
  handler: async (ctx, args) => {
    const current = await ctx.runQuery(api.public.load);
    if (current === 0) {
      return;
    }
    for (let i = 0; i < args.count; i++) {
      await ctx.runMutation(internal.public.resetMutation);
    }
  },
});

export const resetMutation = internalMutation({
  handler: async (ctx) => {
    for (const doc of await ctx.db.query("counter").collect()) {
      await ctx.db.delete(doc._id);
    }
  },
});
