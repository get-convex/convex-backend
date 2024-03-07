import { GenericId, v } from "convex/values";
import { queryGeneric } from "../secretSystemTables";
import { SystemTableNames } from "convex/server";

export default queryGeneric({
  args: { id: v.string() },
  handler: async function ({ db }, args) {
    const id = args.id as GenericId<string>;
    try {
      return await db.get(id);
    } catch (e) {
      return await db.system.get(id as GenericId<SystemTableNames>);
    }
  },
});
