import { useRouter } from "next/router";
import { useTeams } from "api/teams";
import { PROVISION_PROD_PAGE_NAME } from "@common/lib/deploymentContext";
import { useProjectById } from "api/projects";

export function useDeploymentUris(
  projectId: number,
  projectSlug: string,
  teamSlug?: string,
) {
  const router = useRouter();
  const subroute =
    router.route.split("/t/[team]/[project]/[deploymentName]")[1] || "/";
  const { selectedTeamSlug } = useTeams();

  const project = useProjectById(projectId);
  const prodDeploymentName = project?.prodDeploymentName;
  const devDeploymentName = project?.devDeploymentName;

  const projectURI = `/t/${teamSlug || selectedTeamSlug}/${projectSlug}`;

  const prodHref = prodDeploymentName
    ? `${projectURI}/${prodDeploymentName}${subroute}`
    : `${projectURI}/${PROVISION_PROD_PAGE_NAME}`;
  const devHref = devDeploymentName
    ? `${projectURI}/${devDeploymentName}${subroute}`
    : undefined;

  const isProdDefault = !devDeploymentName;

  return {
    isLoading: !project,
    isProdDefault,
    prodHref,
    devHref,
    defaultHref: isProdDefault ? prodHref : devHref,
    generateHref: (deploymentName: string) =>
      `${projectURI}/${deploymentName}${subroute}`,
  };
}
