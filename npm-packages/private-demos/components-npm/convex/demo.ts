import { mutation } from "./_generated/server";

export const insertMessage = mutation({
  args: {},
  handler: async (ctx) => {
    await ctx.db.insert("messages", {
      author: "Nicolas",
      body: "Hello world",
    });
  },
});
