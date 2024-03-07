import { GenericDocument, SystemTableNames } from "convex/server";
import { queryGeneric } from "../secretSystemTables";
import { Id } from "../../_generated/dataModel";
import { v } from "convex/values";

export default queryGeneric({
  args: {
    // We don't validate with v.id here to ensure we can catch ID validation errors inside of the function.
    ids: v.array(
      v.object({
        id: v.string(),
        tableName: v.string(),
      }),
    ),
  },
  handler: async function (
    { db },
    { ids },
  ): Promise<(GenericDocument | null)[]> {
    return await Promise.all(
      ids.map(async ({ id, tableName }) => {
        let normalizedId = null;
        if (tableName.startsWith("_")) {
          normalizedId = db.system.normalizeId(
            tableName as SystemTableNames,
            id,
          );
        } else {
          normalizedId = db.normalizeId(tableName, id);
        }

        if (normalizedId === null) {
          return null;
        }
        if (tableName.startsWith("_")) {
          return await db.system.get(normalizedId as Id<SystemTableNames>);
        }
        return await db.get(normalizedId);
      }),
    );
  },
});
