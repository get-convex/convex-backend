import { useRouter } from "next/router";
import { SWRConfiguration } from "swr";
import { useInitialData } from "hooks/useServerSideData";
import { useCurrentTeam } from "./teams";
import { useBBMutation, useBBQuery } from "./api";

export function useCurrentProject() {
  const team = useCurrentTeam();
  const projects = useProjects(team?.id);
  const { query } = useRouter();
  const { project: projectSlug } = query;
  return projects?.find((p) => p.slug === projectSlug);
}

export function useProjectById(teamId?: number, projectId?: number) {
  const projects = useProjects(teamId);
  return projects?.find((p) => p.id === projectId);
}

export function useProjects(
  teamId?: number,
  refreshInterval?: SWRConfiguration["refreshInterval"],
) {
  const [initialData] = useInitialData();
  const { data } = useBBQuery({
    path: "/teams/{team_id}/projects",
    pathParams: {
      team_id: teamId?.toString() || "",
    },
    // If initial data has been loaded via SSR, we don't need to load projects.
    swrOptions: { refreshInterval, revalidateOnMount: !initialData },
  });
  return data;
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
