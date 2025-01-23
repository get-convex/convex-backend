import { query, internalMutation, mutation } from "./_generated/server";

export const list = query(async (ctx) => {
  const messages = await ctx.db.query("messages").collect();
  for (const message of messages) {
    if (message.format === "dall-e") {
      message.body = await ctx.storage.getUrl(message.body);
    }
  }
  return messages;
});

export const sendDallEMessage = internalMutation(
  async (ctx, { body, author, prompt }) => {
    const message = { body, author, format: "dall-e", prompt };
    await ctx.db.insert("messages", message);
  },
);

export const send = mutation(async (ctx, { body, author }) => {
  const message = { body, author, format: "text" };
  await ctx.db.insert("messages", message);
});
