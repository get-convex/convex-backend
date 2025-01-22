import { v } from "convex/values";
import { mutation } from "./_generated/server";

export const addItem = mutation({
  args: { text: v.string() },
  handler: async (ctx, args) => {
    await ctx.db.insert("tasks", { text: args.text });
    await trackChange(ctx, "addItem");
  },
});

async function trackChange(ctx, type) {
  await ctx.db.insert("changes", { type });
}
