import { mutation } from "./_generated/server";
import { v } from "convex/values";

export default mutation({
  args: {
    body: v.string(),
    author: v.string(),
  },
  handler: async ({ db }, { body, author }) => {
    const message = { body, author, format: "text" };
    await db.insert("messages", message);
  },
});

export const sendImage = mutation({
  args: {
    storageId: v.id("_storage"),
    author: v.string(),
  },
  handler: async ({ db }, { storageId, author }) => {
    const message = { body: storageId, author, format: "image" };
    await db.insert("messages", message);
  },
});
