import { queryPrivateSystem } from "../secretSystemTables";
import { v } from "convex/values";

export const nextScheduledJobTimestamp = queryPrivateSystem({
  args: {
    componentId: v.optional(v.union(v.string(), v.null())),
    udfPath: v.optional(v.string()),
  },
  handler: async function ({ db }, { udfPath }): Promise<bigint | null> {
    if (udfPath === undefined) {
      const nextJob = await db
        .query("_scheduled_jobs")
        .withIndex("by_next_ts", (q) => q.gt("nextTs", null))
        .filter((q) => q.eq(q.field("state"), { type: "pending" }))
        .order("asc")
        .first();
      return nextJob?.nextTs ?? null;
    }

    const nextJob = await db
      .query("_scheduled_jobs")
      .withIndex("by_udf_path_and_next_event_ts", (q) =>
        q.eq("udfPath", udfPath).gt("nextTs", null),
      )
      .filter((q) => q.eq(q.field("state"), { type: "pending" }))
      .order("asc")
      .first();
    return nextJob?.nextTs ?? null;
  },
});
