import { queryPrivateSystem } from "../secretSystemTables";
import { CronJobLog } from "./common";

export default queryPrivateSystem({
  args: {},
  handler: async ({ db }): Promise<CronJobLog[]> => {
    const logs: CronJobLog[] = await db.query("_cron_job_logs").collect();
    return logs;
  },
});
