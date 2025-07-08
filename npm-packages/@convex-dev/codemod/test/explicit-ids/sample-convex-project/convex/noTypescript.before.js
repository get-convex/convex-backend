import { mutation } from "./_generated/server";

export const sampleMutation = mutation({
  args: {},
  handler: async (ctx) => {
    const id = await ctx.db.insert("documents", {
      name: "test",
    });

    await ctx.db.get(id);

    await ctx.db.replace(id, {
      name: "test2",
    });

    await ctx.db.patch(id, {
      name: "test3",
    });

    await ctx.db.delete(id);
  },
});
