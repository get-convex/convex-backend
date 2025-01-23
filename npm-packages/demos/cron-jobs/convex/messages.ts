import { internalMutation, mutation, query } from "./_generated/server";

export const list = query(async (ctx) => {
  return await ctx.db.query("messages").collect();
});

export const send = mutation(async (ctx, { body, author }) => {
  const message = { body, author };
  await ctx.db.insert("messages", message);
});

export const clearAll = internalMutation(async (ctx) => {
  for (const message of await ctx.db.query("messages").collect()) {
    await ctx.db.delete(message._id);
  }
});
