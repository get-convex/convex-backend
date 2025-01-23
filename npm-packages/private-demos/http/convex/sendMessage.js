import { mutation } from "./_generated/server";

export default mutation(async ({ db }, { body, author }) => {
  const message = { body, author, format: "text" };
  await db.insert("messages", message);
});

export const sendImage = mutation(async ({ db }, { storageId, author }) => {
  const message = { body: storageId, author, format: "image" };
  await db.insert("messages", message);
});
