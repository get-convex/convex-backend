import { mutation } from "./_generated/server";

export const init = mutation({
  args: {},
  handler: async (ctx, _args) => {
    const users = await ctx.db.query("users").collect();
    if (users.length) {
      return;
    }
    await ctx.db.insert("users", {
      name: "User 1",
      email: "user1@example.com",
    });
  },
});

export const clearAll = mutation({
  args: {},
  handler: async (ctx, _args) => {
    for (const message of await ctx.db.query("messages").collect()) {
      await ctx.db.delete(message._id);
    }
    for (const conversation of await ctx.db.query("conversations").collect()) {
      await ctx.db.delete(conversation._id);
    }
    for (const conversationMember of await ctx.db
      .query("conversationMembers")
      .collect()) {
      await ctx.db.delete(conversationMember._id);
    }
  },
});
