import { v } from "convex/values";
import { mutation, MutationCtx } from "./_generated/server";

export const addItem = mutation({
  args: { text: v.string() },
  handler: async (ctx, args) => {
    await ctx.db.insert("tasks", { text: args.text });
    await trackChange(ctx, "addItem");
  },
});

async function trackChange(ctx: MutationCtx, type: "addItem" | "removeItem") {
  await ctx.db.insert("changes", { type });
}
