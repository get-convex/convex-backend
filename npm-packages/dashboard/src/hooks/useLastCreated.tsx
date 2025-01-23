import { useTeams } from "api/teams";
import { useProjects } from "api/projects";
import { useDeployments } from "api/deployments";

export function useLastCreatedTeam() {
  const { teams } = useTeams();
  return teams?.at(-1);
}

export function useLastCreatedProject() {
  const team = useLastCreatedTeam();
  const projects = useProjects(team?.id);
  return projects?.at(-1);
}

export function useLastCreatedDeployment() {
  const project = useLastCreatedProject();
  const { deployments } = useDeployments(project?.id);
  return deployments?.at(-1);
}
