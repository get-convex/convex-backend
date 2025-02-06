import useSWR, { SWRConfiguration } from "swr";
import useSWRInfinite from "swr/infinite";

import { toast } from "dashboard-common/lib/utils";
import { Button } from "dashboard-common/elements/Button";
import { SymbolIcon } from "@radix-ui/react-icons";
import { captureMessage } from "@sentry/nextjs";
import flatMap from "lodash/flatMap";
import { useCallback } from "react";
import { AuditLogAction, AuditLogEventResponse } from "generatedApi";
import { LocalDevCallout } from "dashboard-common/elements/Callout";
import { ProjectEnvVarConfig } from "dashboard-common/features/settings/lib/types";
import { fetchWithAuthHeader, useAuthHeader } from "./fetching";
import { useMutation } from "./useMutation";

export function useDeletePreviewDeployment(projectId?: number) {
  return useMutation<{ identifier: string }>({
    url: `/api/dashboard/projects/${projectId}/delete_preview_deployment`,
    mutateKey: `/api/dashboard/projects/${projectId}/instances`,
    successToast: "Deleted preview deployment.",
  });
}

export type ProjectEnvironmentVariable = {
  name: string;
  value: string;
};

export function useProjectEnvironmentVariables(
  projectId?: number,
  refreshInterval?: SWRConfiguration["refreshInterval"],
): { configs: ProjectEnvVarConfig[] } | undefined {
  const { data } = useSWR<{ configs: ProjectEnvVarConfig[] }>(
    projectId
      ? `/api/dashboard/projects/${projectId}/environment_variables/list`
      : undefined,
    null,
    // If initial data has been loaded via SSR, we don't need to load projects.
    { refreshInterval },
  );
  return data;
}

export function useUpdateProjectEnvVars(projectId?: number) {
  return useMutation<{
    changes: {
      oldVariable: ProjectEnvironmentVariable | null;
      newConfig: ProjectEnvVarConfig | null;
    }[];
  }>({
    url: `/api/dashboard/projects/${projectId}/environment_variables/update_batch`,
    mutateKey: `/api/dashboard/projects/${projectId}/environment_variables/list`,
    successToast: "Environment variables updated.",
  });
}

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

// To test that this works
// set the following in your .env.local:
// NEXT_PUBLIC_VERCEL_GIT_COMMIT_SHA=<SHA_THAT_ISN'T_THE_LATEST>
// VERCEL_TOKEN=<VERCEL_ACCESS_TOKEN>
export function useDashboardVersion() {
  const { data, error } = useSWR<{ sha?: string | null }>("/api/version", {
    // Refresh every hour.
    refreshInterval: 1000 * 60 * 60,
    // Refresh on focus at most every 10 minutes.
    focusThrottleInterval: 1000 * 60 * 10,
    shouldRetryOnError: false,
    fetcher: dashboardVersionFetcher,
  });

  const currentSha = process.env.NEXT_PUBLIC_VERCEL_GIT_COMMIT_SHA;
  if (!error && data?.sha && currentSha && data?.sha !== currentSha) {
    toast(
      "info",
      <div className="flex flex-col">
        A new version of the Convex dashboard is available! Refresh this page to
        update.
        <LocalDevCallout tipText="In local development, the local git sha is being compared to the latest production deployment's sha." />
        <Button
          className="ml-auto w-fit items-center"
          inline
          size="xs"
          icon={<SymbolIcon />}
          // Make the href the current page so that the page refreshes.
          onClick={() => window.location.reload()}
        >
          Refresh
        </Button>
      </div>,
      "dashboardVersion",
      false,
    );
  }
}

// Custom fetcher because we're using Vercel functions and not big brain.
const dashboardVersionFetcher = async (url: string) => {
  const res = await fetch(url);
  if (!res.ok) {
    try {
      const { error } = await res.json();
      captureMessage(error);
    } catch (e) {
      captureMessage("Failed to fetch dashboard version information.");
    }
    throw new Error("Failed to fetch dashboard version information.");
  }
  return res.json();
};
