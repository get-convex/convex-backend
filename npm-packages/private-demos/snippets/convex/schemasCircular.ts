import { mutation } from "./_generated/server";

export default mutation({
  handler: async (ctx) => {
    const preferencesId = await ctx.db.insert("preferences", {});
    const userId = await ctx.db.insert("users", { preferencesId });
    await ctx.db.patch(preferencesId, { userId });
  },
});
