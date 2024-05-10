import { CronJob, CronJobWithLastRun } from "./common";
import { queryPrivateSystem } from "../secretSystemTables";
import { v } from "convex/values";
export default queryPrivateSystem({
  args: { componentId: v.optional(v.union(v.string(), v.null())) },
  handler: async ({ db }): Promise<CronJobWithLastRun[]> => {
    const jobs: CronJob[] = await db.query("_cron_jobs").collect();
    const jobsWithLastRun: CronJobWithLastRun[] = [];

    for (const job of jobs) {
      const lastRun = await db
        .query("_cron_job_logs")
        .withIndex("by_name_and_ts", (q) => q.eq("name", job.name))
        .order("desc")
        .first();
      jobsWithLastRun.push({
        ...job,
        lastRun: lastRun || null,
      });
    }

    return jobsWithLastRun;
  },
});
