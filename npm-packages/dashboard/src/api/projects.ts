import { useRouter } from "next/router";
import { SWRConfiguration } from "swr";
import { useMemo, useEffect, useRef, useState } from "react";
import {
  PaginatedProjectsResponse,
  ProjectDetails,
  operations,
} from "generatedApi";
import { useGlobalLocalStorage } from "@common/lib/useGlobalLocalStorage";
import { useDebounce } from "react-use";
import { createInfiniteHook } from "swr-openapi";
import { useAuthHeader } from "hooks/fetching";
import flatMap from "lodash/flatMap";
import { useCurrentTeam } from "./teams";
import { useBBMutation, useBBQuery, client } from "./api";

const useInfinite = createInfiniteHook(client, "big-brain");

export function useCurrentProject() {
  const team = useCurrentTeam();
  const { query } = useRouter();
  const { project: projectSlug } = query;
  return useProjectBySlug(team?.id, projectSlug as string);
}

export function useProjectById(projectId: number | undefined) {
  const { data } = useBBQuery({
    path: "/projects/{project_id}",
    pathParams: {
      project_id: projectId?.toString() || "",
    },
  });
  return data;
}

export function useProjectBySlug(
  teamId: number | undefined,
  projectSlug: string | undefined,
) {
  const { data, isLoading } = useBBQuery({
    path: "/teams/{team_id}/projects/{project_slug}",
    pathParams: {
      team_id: teamId || 0,
      project_slug: projectSlug || "",
    },
  });
  if (isLoading) {
    return undefined;
  }
  return data;
}

export function usePaginatedProjects(
  teamId: number | undefined,
  options: {
    cursor?: string;
    q?: string;
  },
  refreshInterval?: SWRConfiguration["refreshInterval"],
): (PaginatedProjectsResponse & { isLoading: boolean }) | undefined {
  const [pageSize] = useGlobalLocalStorage("projectsPageSize", 25);

  const queryParams = useMemo(
    () =>
      ({
        cursor: options.cursor,
        limit: pageSize,
        q: options.q,
      }) satisfies operations["get_projects_for_team"]["parameters"]["query"],
    [options, pageSize],
  );

  const { data, isLoading } = useBBQuery({
    path: "/teams/{team_id}/projects",
    pathParams: {
      team_id: teamId?.toString() || "",
    },
    queryParams,
    swrOptions: { refreshInterval },
  });

  if (data === undefined) {
    return undefined;
  }

  // If it's an array (simple response), convert to paginated format
  if (Array.isArray(data)) {
    return {
      items: data,
      pagination: {
        hasMore: false,
      },
      isLoading,
    };
  }

  return { ...data, isLoading };
}

// Returns all projects (unpaginated) - for backward compatibility
export function useProjects(
  teamId: number | undefined,
  refreshInterval?: SWRConfiguration["refreshInterval"],
): ProjectDetails[] | undefined {
  const { data: allProjects } = useBBQuery({
    path: "/teams/{team_id}/projects",
    pathParams: {
      team_id: teamId?.toString() || "",
    },
    swrOptions: { refreshInterval },
  });

  if (allProjects === undefined) {
    return undefined;
  }

  if (!Array.isArray(allProjects)) {
    throw new Error("Expected array of projects");
  }

  return allProjects;
}

/**
 * Hook for infinite scroll pagination of projects with search support
 * Returns paginated projects data with loading state and pagination controls
 */
export function useInfiniteProjects(teamId: number, searchQuery: string = "") {
  const authHeader = useAuthHeader();
  const [pageSize] = useGlobalLocalStorage("projectsPageSize", 25);
  const [debouncedQuery, setDebouncedQuery] = useState("");

  // Debounce search query (300ms delay)
  useDebounce(
    () => {
      setDebouncedQuery(searchQuery);
    },
    300,
    [searchQuery],
  );

  const { data, isLoading, setSize } = useInfinite(
    "/teams/{team_id}/projects",
    (
      pageIndex: number,
      previousPageData: PaginatedProjectsResponse | null,
    ): any => {
      // Stop if we've reached the end (but allow first page)
      if (
        pageIndex > 0 &&
        previousPageData &&
        !previousPageData.pagination.hasMore
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
            limit: pageSize,
            cursor:
              pageIndex > 0 && previousPageData
                ? previousPageData.pagination.nextCursor
                : undefined,
            ...(debouncedQuery.trim() ? { q: debouncedQuery.trim() } : {}),
          },
        },
      };
    },
  );

  // Manual reset when query changes
  const prevQuery = useRef(debouncedQuery);
  useEffect(() => {
    if (prevQuery.current !== debouncedQuery) {
      prevQuery.current = debouncedQuery;
      void setSize(1);
    }
  }, [debouncedQuery, setSize]);

  const projects = useMemo(
    () => flatMap(data?.map((page) => page.items)),
    [data],
  );

  const hasMore = data?.[data.length - 1]?.pagination.hasMore ?? false;

  return {
    projects,
    isLoading,
    hasMore,
    loadMore: () => {
      if (hasMore && !isLoading) {
        void setSize((prevSize) => prevSize + 1);
      }
    },
    debouncedQuery,
    pageSize,
  };
}

export function useCreateProject(teamId?: number) {
  return useBBMutation({
    path: "/create_project",
    pathParams: undefined,
    mutateKey: "/teams/{team_id}/projects",
    mutatePathParams: {
      team_id: teamId?.toString() || "",
    },
    googleAnalyticsEvent: "create_project_dash",
  });
}

export function useUpdateProject(projectId: number) {
  return useBBMutation({
    path: "/projects/{project_id}",
    pathParams: {
      project_id: projectId.toString(),
    },
    successToast: "Project updated.",
    method: "put",
  });
}

export function useDeleteProject(
  teamId?: number,
  projectId?: number,
  projectName?: string,
) {
  return useBBMutation({
    path: "/delete_project/{project_id}",
    pathParams: {
      project_id: projectId?.toString() || "",
    },
    mutateKey: "/teams/{team_id}/projects",
    mutatePathParams: {
      team_id: teamId?.toString() || "",
    },
    successToast: projectName ? `Deleted project: ${projectName}.` : undefined,
    redirectTo: "/",
  });
}

export function useDeleteProjects(teamId: number | undefined) {
  return useBBMutation({
    path: "/delete_projects",
    pathParams: undefined,
    mutateKey: "/teams/{team_id}/projects",
    mutatePathParams: {
      team_id: teamId?.toString() || "",
    },
    successToast: "Projects deleted.",
  });
}
