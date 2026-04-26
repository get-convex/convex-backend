import { queryPrivateSystem } from "../secretSystemTables";
import { Doc } from "../../_generated/dataModel";

/**
 * Read the singleton `_periodic_backup_config` row, if any. Returns `null`
 * when periodic backups are disabled (no row exists).
 */
export const get = queryPrivateSystem("ViewBackups")({
  args: {},
  handler: async function ({
    db,
  }): Promise<Doc<"_periodic_backup_config"> | null> {
    return await db.query("_periodic_backup_config").unique();
  },
});
