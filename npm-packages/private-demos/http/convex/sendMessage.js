import { mutation } from "./_generated/server";

export default mutation({
  handler: async ({ db }, { body, author }) => {
    const message = { body, author, format: "text" };
    await db.insert("messages", message);
  },
});

export const sendImage = mutation({
  handler: async ({ db }, { storageId, author }) => {
    const message = { body: storageId, author, format: "image" };
    await db.insert("messages", message);
  },
});
