import { queryPrivateSystem } from "../secretSystemTables";
import { v } from "convex/values";

export const nextScheduledJobTimestamp = queryPrivateSystem({
  args: {
    componentId: v.optional(v.union(v.string(), v.null())),
  },
  handler: async function ({ db }): Promise<bigint | null> {
    const nextJob = await db
      .query("_scheduled_jobs")
      .withIndex("by_next_ts", (q) => q.gt("nextTs", null))
      .filter((q) => q.eq(q.field("state"), { type: "pending" }))
      .order("asc")
      .first();
    return nextJob?.nextTs ?? null;
  },
});
