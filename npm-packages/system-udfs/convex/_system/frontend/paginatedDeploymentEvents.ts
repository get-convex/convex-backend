import { paginationOptsValidator } from "convex/server";
import { queryPrivateSystem } from "../secretSystemTables";
import { v } from "convex/values";
import { maximumBytesRead, maximumRowsRead } from "../paginationLimits";

/**
 * Paginated query for the deployment events from most recent to least recent
 */
export default queryPrivateSystem({
  args: {
    paginationOpts: paginationOptsValidator,
    filters: v.object({
      minDate: v.number(),
      maxDate: v.optional(v.number()),
      authorMemberIds: v.optional(v.array(v.int64())),
      actions: v.optional(v.array(v.string())),
    }),
  },
  handler: async function ({ db }, { paginationOpts, filters }) {
    const paginatedResults = await db
      .query("_deployment_audit_log")
      .withIndex("by_creation_time", (q) => {
        const partial = q.gte("_creationTime", filters.minDate);

        return filters.maxDate
          ? partial.lte("_creationTime", filters.maxDate)
          : partial;
      })
      .order("desc")
      .filter((q) => {
        const queryFilters = [];
        if (filters.authorMemberIds !== undefined) {
          queryFilters.push(
            q.or(
              ...filters.authorMemberIds.map((memberId) =>
                q.eq(memberId, q.field("member_id")),
              ),
            ),
          );
        }
        if (filters.actions != undefined) {
          queryFilters.push(
            q.or(
              ...filters.actions.map((action) =>
                q.eq(action, q.field("action")),
              ),
            ),
          );
        }
        return q.and(...queryFilters);
      })
      .paginate({
        ...paginationOpts,
        maximumBytesRead,
        maximumRowsRead,
      });

    return paginatedResults;
  },
});
