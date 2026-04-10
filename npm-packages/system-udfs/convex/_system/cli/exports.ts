import { Export } from "../frontend/common";
import { queryPrivateSystem, requireOperation } from "../secretSystemTables";

export const getLatest = queryPrivateSystem({
  args: {},
  handler: async ({ db }): Promise<Export | null> => {
    requireOperation("ViewBackups");
    return await db
      .query("_exports")
      .withIndex("by_requestor", (q) => q.eq("requestor", "snapshotExport"))
      .order("desc")
      .first();
  },
});
