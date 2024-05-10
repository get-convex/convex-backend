import { v } from "convex/values";
import { performOp } from "../../syscall";
import { queryPrivateSystem } from "../secretSystemTables";

/**
 * Returns an object mapping the table numbers to table names
 * (e.g. {"1": "users"})
 */
export default queryPrivateSystem({
  args: { componentId: v.optional(v.union(v.string(), v.null())) },
  handler: async () => {
    return performOp("getTableMappingWithoutSystemTables");
  },
});
