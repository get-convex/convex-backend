import { Integration } from "./common";
import { queryPrivateSystem, requireOperation } from "../secretSystemTables";
export default queryPrivateSystem({
  args: {},
  handler: async ({ db }): Promise<Integration[]> => {
    requireOperation("ViewIntegrations");
    return await db.query("_log_sinks").collect();
  },
});
