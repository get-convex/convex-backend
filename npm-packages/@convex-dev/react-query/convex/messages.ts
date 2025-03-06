import { mutation } from "./_generated/server.js";
import { query } from "./_generated/server.js";
import { Doc } from "./_generated/dataModel.js";

export const list = query({
  handler: async (ctx): Promise<Doc<"messages">[]> => {
    return await ctx.db.query("messages").collect();
  },
});

export const count = query({
  handler: async (ctx): Promise<string> => {
    const messages = await ctx.db.query("messages").take(1001);
    return messages.length === 1001 ? "1000+" : `${messages.length}`;
  },
});

export const send = mutation({
  handler: async (ctx, { body, author }: { body: string; author: string }) => {
    const message = { body, author };
    await ctx.db.insert("messages", message);
  },
});
