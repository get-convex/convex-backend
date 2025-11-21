import { query, mutation } from "./_generated/server";

export const partialEscapeSequenceReturn = query({
  args: {},
  handler: async () => {
    return "\ud83c...";
  },
});

export const partialEscapeSequenceConsoleLog = query({
  args: {},
  handler: async () => {
    console.log("\ud83c...");
  },
});

export const partialEscapeSequenceDbInsert = mutation({
  args: {},
  handler: async (ctx) => {
    return ctx.db.insert("table", {
      body: "\ud83c...",
    });
  },
});
