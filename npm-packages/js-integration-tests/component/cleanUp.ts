import { ALL_TABLE_NAMES } from "./schema";
import { mutation } from "./_generated/server";

export default mutation(async (ctx) => {
  for (const table of ALL_TABLE_NAMES) {
    const docs = await ctx.db.query(table).collect();
    for (const doc of docs) {
      await ctx.db.delete(doc._id);
    }
  }
  const docs = await ctx.db.system.query("_storage").collect();
  for (const doc of docs) {
    await ctx.storage.delete(doc._id);
  }
});
