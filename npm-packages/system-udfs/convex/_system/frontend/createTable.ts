import { v } from "convex/values";
import { mutationGeneric } from "convex/server";

export default mutationGeneric({
  args: { table: v.string() },
  handler: async ({ db }, { table }) => {
    // We don't have an "official" way to create a table without inserting documents,
    // but inserting a document and deleting it in the same transaction is one way to do this.
    const id = await db.insert(table, {});
    await db.delete(id);
  },
});
