import { v } from "convex/values";
import { queryPrivateSystem } from "../secretSystemTables";

export default queryPrivateSystem({
  args: { importId: v.id("_snapshot_imports") },
  handler: async ({ db }, args) => {
    return await db.get(args.importId);
  },
});
