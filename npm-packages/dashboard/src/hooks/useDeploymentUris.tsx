import { PROVISION_PROD_PAGE_NAME } from "dashboard-common";
import { useRouter } from "next/router";
import { useTeams } from "api/teams";
import { useDefaultDevDeployment, useDeployments } from "api/deployments";
import { useLaunchDarkly } from "./useLaunchDarkly";

export function useDeploymentUris(
  projectId: number,
  projectSlug: string,
  teamSlug?: string,
) {
  const { localDeployments } = useLaunchDarkly();
  const router = useRouter();
  const subroute =
    router.route.split("/t/[team]/[project]/[deploymentName]")[1] || "/";
  const { selectedTeamSlug } = useTeams();

  const { deployments } = useDeployments(projectId);

  const projectURI = `/t/${teamSlug || selectedTeamSlug}/${projectSlug}`;

  const prodDeployment =
    deployments &&
    deployments.find((deployment) => deployment.deploymentType === "prod");
  const prodHref = prodDeployment
    ? `${projectURI}/${prodDeployment.name}${subroute}`
    : `${projectURI}/${PROVISION_PROD_PAGE_NAME}`;
  const devDeployment = useDefaultDevDeployment(projectId, localDeployments);
  const devHref = devDeployment
    ? `${projectURI}/${devDeployment.name}${subroute}`
    : undefined;

  const isProdDefault = !devDeployment;

  return {
    isLoading: !deployments,
    isProdDefault,
    prodHref,
    devHref,
    defaultHref: isProdDefault ? prodHref : devHref,
  };
}
