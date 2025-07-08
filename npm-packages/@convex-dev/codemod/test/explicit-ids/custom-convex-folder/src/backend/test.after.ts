import { v } from "convex/values";
import { query } from "./_generated/server";

export const get = query({
  args: {
    id: v.id("documents"),
  },
  handler: async (ctx, { id }) => {
    return await ctx.db.get("documents", id);
  },
});
