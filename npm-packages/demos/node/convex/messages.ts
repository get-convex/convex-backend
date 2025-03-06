import { mutation } from "./_generated/server";
import { query } from "./_generated/server";
import { Doc } from "./_generated/dataModel";

export const list = query({
  handler: async (ctx): Promise<Doc<"messages">[]> => {
    return await ctx.db.query("messages").collect();
  },
});

export const send = mutation(
  async (ctx, { body, author }: { body: string; author: string }) => {
    const message = { body, author };
    await ctx.db.insert("messages", message);
  },
);
