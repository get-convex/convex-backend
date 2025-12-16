import { useTeams } from "api/teams";
import { usePaginatedProjects } from "api/projects";
import { useDeployments } from "api/deployments";

export function useLastCreatedTeam() {
  const { teams } = useTeams();
  return teams?.at(-1);
}

export function useLastCreatedProject() {
  const team = useLastCreatedTeam();
  const paginatedProjects = usePaginatedProjects(team?.id, {
    cursor: undefined,
    q: undefined,
  });

  if (!paginatedProjects) return undefined;

  const projects = paginatedProjects ? paginatedProjects.items : [];
  return projects.at(-1) || null;
}

export function useLastCreatedDeployment() {
  const project = useLastCreatedProject();
  const { deployments } = useDeployments(project?.id);
  if (project === null) {
    return null;
  }
  if (project === undefined || deployments === undefined) return undefined;
  return deployments?.at(-1) || null;
}
