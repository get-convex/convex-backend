import flatMap from "lodash/flatMap";
import { AuditLogAction, AuditLogEventResponse } from "generatedApi";
import { createInfiniteHook } from "swr-openapi";
import { client } from "api/api";
import { useAuthHeader } from "hooks/fetching";

const useInfinite = createInfiniteHook(client, "big-brain");

export function useTeamAuditLog(
  teamId: number,
  {
    from,
    to,
    memberId,
    action,
  }: {
    from: number;
    to: number;
    memberId: string | null;
    action: AuditLogAction | null;
  },
): {
  entries?: AuditLogEventResponse[];
  isLoading: boolean;
  loadNextPage: () => void;
  hasMore: boolean;
} {
  const authHeader = useAuthHeader();

  const { data, isLoading, setSize } = useInfinite(
    "/teams/{team_id}/get_audit_log_events",
    (
      pageIndex: number,
      previousPageData: {
        events: AuditLogEventResponse[];
        cursor?: string;
      } | null,
    ) => {
      if (
        previousPageData &&
        (!previousPageData.events || !previousPageData.cursor)
      ) {
        return null;
      }

      return {
        headers: {
          Authorization: authHeader,
          "Convex-Client": "dashboard-0.0.0",
        },
        params: {
          path: {
            team_id: teamId.toString(),
          },
          query: {
            from,
            to,
            ...(memberId ? { member_id: memberId } : {}),
            ...(action ? { action } : {}),
            cursor: pageIndex > 0 ? previousPageData?.cursor : undefined,
          },
        },
      };
    },
  );

  return {
    entries: flatMap(
      data?.map((page) => page.events),
      (entry) => entry,
    ),
    loadNextPage: () => setSize((size) => size + 1),
    isLoading,
    hasMore: !!data?.[data.length - 1]?.cursor,
  };
}
