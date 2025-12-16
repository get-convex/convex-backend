// @snippet start list
import { query, mutation } from "./_generated/server";
import { v } from "convex/values";

export const list = query({
  args: {},
  handler: async ({ db }) => {
    return await db.query("messages").collect();
  },
});
// @snippet end list

// @snippet start send
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
// @snippet end send
