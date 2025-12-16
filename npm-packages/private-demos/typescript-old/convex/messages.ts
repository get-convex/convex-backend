import { mutation } from "./_generated/server";
import { query } from "./_generated/server";
import { Doc } from "./_generated/dataModel";
import { v } from "convex/values";

export const list = query({
  args: {},
  handler: async ({ db }): Promise<Doc<"messages">[]> => {
    return await db.query("messages").collect();
  },
});

export const send = mutation({
  args: {
    body: v.string(),
    author: v.string(),
  },
  handler: async ({ db }, { body, author }) => {
    const message = { body, author };
    await db.insert("messages", message);
  },
});
