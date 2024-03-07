import { Sink } from "./common";
import { queryPrivateSystem } from "../secretSystemTables";
export default queryPrivateSystem({
  args: {},
  handler: async ({ db }): Promise<Sink[]> => {
    return await db.query("_log_sinks").collect();
  },
});
