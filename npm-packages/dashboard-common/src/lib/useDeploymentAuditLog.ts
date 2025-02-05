import * as Sentry from "@sentry/nextjs";
import { usePaginatedQuery } from "convex/react";
import udfs from "udfs";
import { Doc } from "system-udfs/convex/_generated/dataModel";
import { useEffect, useContext } from "react";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";

export type DeploymentAuditLogEvent = Doc<"_deployment_audit_log"> & {
  memberName: string;
  metadata?: {
    schema?: null | {
      previous_schema: null | string;
      next_schema: null | string;
    };
  };
};

function getMemberName(
  auditLogEvent: Doc<"_deployment_audit_log">,
  teamMembers: { id: number; name?: string | null; email?: string }[],
): string {
  if (auditLogEvent.member_id === null) {
    return "Convex";
  }

  const member = teamMembers.find(
    (m) => BigInt(m.id) === auditLogEvent.member_id,
  );
  return member?.name || member?.email || "Unknown member";
}

function processDeploymentAuditLogEvent(
  auditLogEvent: Doc<"_deployment_audit_log">,
  teamMembers: {
    id: number;
    name?: string | null;
    email?: string;
  }[],
): DeploymentAuditLogEvent | null {
  switch (auditLogEvent.action) {
    case "create_environment_variable":
    case "update_environment_variable":
    case "delete_environment_variable":
    case "replace_environment_variable":
    case "push_config":
    case "push_config_with_components":
    case "change_deployment_state":
    case "build_indexes":
    case "clear_tables":
    case "snapshot_import":
      break;
    default:
      // eslint-disable-next-line no-case-declarations, @typescript-eslint/no-unused-vars
      Sentry.captureMessage(
        `Unexpected deployment audit log with action ${
          (auditLogEvent as any).action
        }`,
      );
      return null;
  }

  return {
    ...auditLogEvent,
    memberName: getMemberName(auditLogEvent, teamMembers),
  } as DeploymentAuditLogEvent;
}

function processDeploymentEvents(
  events: Doc<"_deployment_audit_log">[],
  teamMembers: {
    id: number;
    name?: string | null;
    email?: string;
  }[],
): DeploymentAuditLogEvent[] {
  return events
    .map((event) => processDeploymentAuditLogEvent(event, teamMembers))
    .filter((v): v is DeploymentAuditLogEvent => v !== null);
}

export function useDeploymentAuditLogs(
  fromTimestamp?: number,
  filters?: {
    actions: string[];
  },
): DeploymentAuditLogEvent[] | undefined {
  const { useCurrentTeam, useTeamMembers } = useContext(DeploymentInfoContext);
  const team = useCurrentTeam();
  const teamMembers = useTeamMembers(team?.id) || [];

  const initialNumItems = 50;
  const { results, loadMore, status } = usePaginatedDeploymentEvents(
    fromTimestamp
      ? {
          minDate: fromTimestamp,
          actions: filters ? filters.actions : undefined,
        }
      : undefined,
    teamMembers,
    initialNumItems,
  );

  // Load items until we've exhuasted the results.
  useEffect(() => {
    if (status === "CanLoadMore") {
      loadMore(initialNumItems);
    }
  }, [loadMore, status]);
  return results ? results.reverse() : undefined;
}

export type DeploymentAuditLogFilters = {
  minDate: number;
  maxDate?: number;
  authorMemberIds?: bigint[];
  actions?: string[];
};

export function usePaginatedDeploymentEvents(
  filters: DeploymentAuditLogFilters | undefined,
  teamMembers: {
    id: number;
    name?: string | null;
    email?: string;
  }[],
  initialNumItems = 10,
) {
  const { results, ...rest } = usePaginatedQuery(
    udfs.paginatedDeploymentEvents.default,
    filters ? { filters } : "skip",
    {
      initialNumItems,
    },
  );
  return {
    results: processDeploymentEvents(results, teamMembers),
    ...rest,
  };
}
