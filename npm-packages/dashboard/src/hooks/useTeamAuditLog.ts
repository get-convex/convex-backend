import useSWRInfinite from "swr/infinite";
import flatMap from "lodash/flatMap";
import { useCallback } from "react";
import { AuditLogAction, AuditLogEventResponse } from "generatedApi";
import { fetchWithAuthHeader, useAuthHeader } from "./fetching";

// TODO: Refactor when we support query params
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
  const getKey = useCallback(
    (
      pageIndex: number,
      previousPageData: { events: AuditLogEventResponse[]; cursor?: string },
    ) => {
      if (
        previousPageData &&
        (!previousPageData.events || !previousPageData.cursor)
      ) {
        return null;
      }

      let url = `/api/dashboard/teams/${teamId}/get_audit_log_events?from=${from}&to=${to}`;

      if (memberId) {
        url += `&memberId=${memberId}`;
      }
      if (action) {
        url += `&action=${action}`;
      }
      if (pageIndex > 0) {
        url += `&cursor=${previousPageData.cursor}`;
      }
      return url;
    },
    [action, from, memberId, teamId, to],
  );

  const authHeader = useAuthHeader();
  const { data, error, isLoading, setSize } = useSWRInfinite<{
    events: AuditLogEventResponse[];
    cursor: string;
  }>(getKey, (key) => fetchWithAuthHeader([key, authHeader]), {
    use: [],
  });
  if (error) {
    throw error;
  }

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
