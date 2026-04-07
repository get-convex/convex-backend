import { mutation, query } from "./_generated/server";

export const write = mutation({
  args: {},
  handler: async (ctx) => {
    await ctx.db.insert("test", {
      message: "Hello, world!",
    });
  },
});

export const didWrite = query({
  args: {},
  handler: async (ctx) => {
    return (await ctx.db.query("test").first()) !== null;
  },
});
