import { query, mutation } from "./_generated/server";

export const list = query(async (ctx) => {
  return await ctx.db.query("messages").collect();
});

export const send = mutation(async (ctx, { body, author }) => {
  const message = { body, author };
  await ctx.db.insert("messages", message);
});

export const sendImage = mutation(async (ctx, { storageId, author }) => {
  const message = { body: storageId, author, format: "image" };
  await ctx.db.insert("messages", message);
});
