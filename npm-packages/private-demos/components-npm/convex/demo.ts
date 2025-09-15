import { mutation } from "./_generated/server";

export const insertMessage = mutation({
  handler: async (ctx) => {
    await ctx.db.insert("messages", {
      author: "Nicolas",
      body: "Hello world",
    });
  },
});
