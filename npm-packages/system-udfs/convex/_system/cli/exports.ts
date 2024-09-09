import { Export } from "../frontend/common";
import { queryPrivateSystem } from "../secretSystemTables";

export const getLatest = queryPrivateSystem({
  args: {},
  handler: async ({ db }): Promise<Export | null> => {
    return await db.query("_exports").order("desc").first();
  },
});
