import { query, mutation } from "./_generated/server";

export const partialEscapeSequenceReturn = query(async () => {
  return "\ud83c...";
});

export const partialEscapeSequenceConsoleLog = query(async () => {
  console.log("\ud83c...");
});

export const partialEscapeSequenceDbInsert = mutation(async (ctx) => {
  return ctx.db.insert("table", {
    body: "\ud83c...",
  });
});
