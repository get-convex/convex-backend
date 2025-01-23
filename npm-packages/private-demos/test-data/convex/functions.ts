import { action, mutation } from "./_generated/server";
import { api } from "./_generated/api";

export const populate = mutation({
  args: {},
  handler: async (ctx) => {
    await ctx.db.insert("messages", {
      text: "Hello, world!",
      channel: "general",
    });
    await ctx.db.insert("messages", { channel: "smokey", text: "Mama mia" });
    await ctx.db.insert("messages", {
      channel: "private",
      text: "Everything perishes in the universe",
    });

    await ctx.db.insert("users", { name: "Alice" });
    await ctx.db.insert("users", { name: "Nipunn" });

    await ctx.db.insert("maps", { map: "Island with a hidden treasure" });
  },
});

export const populateVirtual = action({
  args: {},
  handler: async (ctx) => {
    await ctx.storage.store(new Blob(["Hello"], { type: "text/plain" }));
    await ctx.scheduler.runAfter(10 * 24 * 3600 * 1000, api.functions.populate);
  },
});
