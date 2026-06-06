import { paginationOptsValidator } from "convex/server";
import { queryPrivateSystem } from "../secretSystemTables";
import { v } from "convex/values";
import { maximumBytesRead, maximumRowsRead } from "../paginationLimits";
import { clampForAuditLogRetention } from "./auditLogRetention";
import { hasDeploymentEventPostFilters } from "./deploymentEventFilters";

export { clampForAuditLogRetention } from "./auditLogRetention";

/**
 * Paginated query for the deployment events from most recent to least recent
 */
export default queryPrivateSystem("ViewAuditLog")({
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
    const minDate = await clampForAuditLogRetention(db, filters.minDate);

    const deploymentEvents = db
      .query("_deployment_audit_log")
      .withIndex("by_creation_time", (q) => {
        const partial = q.gte("_creationTime", minDate);

        return filters.maxDate
          ? partial.lte("_creationTime", filters.maxDate)
          : partial;
      })
      .order("desc");

    const filteredDeploymentEvents = hasDeploymentEventPostFilters(filters)
      ? deploymentEvents
          // eslint-disable-next-line @convex-dev/no-filter-in-query -- we allow filtering by multiple member IDs/actions
          .filter((q) => {
            // FIXME: Note that here, we could use an index for the case where we filter for a single member ID and/or a single action

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
            if (filters.actions !== undefined) {
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
      : deploymentEvents;

    const paginatedResults = await filteredDeploymentEvents.paginate({
      ...paginationOpts,
      maximumBytesRead,
      maximumRowsRead,
    });

    return paginatedResults;
  },
});
