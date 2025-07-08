import { mutation } from "./_generated/server";

export const sampleMutation = mutation({
  args: {},
  handler: async (ctx) => {
    const id = await ctx.db.insert("documents", {
      name: "test",
    });

    await ctx.db.get("documents", id);

    await ctx.db.replace("documents", id, {
      name: "test2",
    });

    await ctx.db.patch("documents", id, {
      name: "test3",
    });

    await ctx.db.delete("documents", id);
  },
});
