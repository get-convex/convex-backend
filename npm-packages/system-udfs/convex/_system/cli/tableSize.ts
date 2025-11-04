import { queryGeneric } from "../secretSystemTables";
import { v } from "convex/values";

export default queryGeneric({
  args: {
    tableName: v.string(),
  },
  handler: async function tableSize({ db }, { tableName }): Promise<number> {
    return await db.query(tableName).count();
  },
});
