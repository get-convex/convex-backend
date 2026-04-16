import { Integration } from "./common";
import { queryPrivateSystem } from "../secretSystemTables";
export default queryPrivateSystem("ViewIntegrations")({
  args: {},
  handler: async ({ db }): Promise<Integration[]> => {
    return await db.query("_log_sinks").collect();
  },
});
