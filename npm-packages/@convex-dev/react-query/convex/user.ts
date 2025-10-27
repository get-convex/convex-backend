import { getAuthUserId } from "@convex-dev/auth/server";
import { Doc } from "./_generated/dataModel";
import { query } from "./_generated/server";

export const getCurrent = query({
  args: {},
  handler: async (ctx): Promise<Doc<"users"> | null> => {
    const userId = await getAuthUserId(ctx);
    if (!userId) {
      throw new Error("Unauthorized");
    }
    return await ctx.db.get(userId);
  },
});
