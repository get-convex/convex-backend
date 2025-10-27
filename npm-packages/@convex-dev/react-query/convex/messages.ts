import { mutation } from "./_generated/server.js";
import { query } from "./_generated/server.js";
import { v } from "convex/values";
import { vv } from "./schema.js";

export const list = query({
  args: {},
  returns: v.array(
    v.object({
      ...vv.doc("messages").fields,
      authorId: v.id("users"),
      authorEmail: v.optional(v.string()),
    }),
  ),
  handler: async (ctx) => {
    const messages = await ctx.db.query("messages").collect();
    return Promise.all(
      messages.map(async (message) => {
        const author = await ctx.db.get(message.author);
        if (!author) {
          throw new Error("Author not found");
        }
        return { ...message, authorId: author._id, authorEmail: author.email };
      }),
    );
  },
});

export const count = query({
  args: {},
  returns: v.string(),
  handler: async (ctx) => {
    const messages = await ctx.db.query("messages").take(1001);
    return messages.length === 1001 ? "1000+" : `${messages.length}`;
  },
});

export const send = mutation({
  args: {
    body: v.string(),
    author: v.id("users"),
  },
  handler: async (ctx, args) => {
    const message = { body: args.body, author: args.author };
    await ctx.db.insert("messages", message);
  },
});
