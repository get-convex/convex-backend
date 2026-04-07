import { ALL_TABLE_NAMES } from "./schema";
// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
import { mutation } from "./_generated/server";
import { components } from "./_generated/api";

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
  // eslint-disable-next-line @typescript-eslint/ban-ts-comment
  // @ts-ignore
  await ctx.runMutation(components.component.cleanUp.default);
});
