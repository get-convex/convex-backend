import { mutation } from "./_generated/server";

export default mutation(async (ctx, { body, author }) => {
  const message = { body, author };
  await ctx.db.insert("messages", message);
});
