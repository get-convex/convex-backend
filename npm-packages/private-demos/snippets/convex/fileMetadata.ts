import { v } from "convex/values";
import { action } from "./_generated/server";

export const getMetadata = action({
  args: { storageId: v.id("_storage") },
  handler: async (ctx, args) => {
    return await ctx.storage.getMetadata(args.storageId);
  },
});
