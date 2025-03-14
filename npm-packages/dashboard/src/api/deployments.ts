import { useRouter } from "next/router";
import { useInitialData } from "hooks/useServerSideData";
import { useProfile } from "./profile";
import { useCurrentProject } from "./projects";
import { useBBMutation, useBBQuery } from "./api";

export function useDeployments(projectId?: number) {
  const [initialData] = useInitialData();
  const { data, isLoading } = useBBQuery({
    path: "/projects/{project_id}/instances",
    pathParams: {
      project_id: projectId?.toString() || "",
    },
    swrOptions: {
      revalidateOnMount: initialData === undefined,
      refreshInterval: 30 * 1000,
    },
  });

  return { deployments: data, isLoading };
}

export function useDefaultDevDeployment(projectId: number | undefined) {
  const member = useProfile();
  const { deployments } = useDeployments(projectId);
  const cloudDev = deployments?.find(
    (d) =>
      d.deploymentType === "dev" &&
      d.kind === "cloud" &&
      d.creator === member?.id,
  );
  const localDev = deployments?.find(
    (d) =>
      d.deploymentType === "dev" &&
      d.kind === "local" &&
      d.creator === member?.id &&
      d.isActive,
  );
  // Prefer local deployments if they exist.
  return localDev ?? cloudDev;
}

export function useCurrentDeployment() {
  const project = useCurrentProject();
  const { deployments, isLoading } = useDeployments(project?.id);
  const { push, query } = useRouter();
  const deployment = deployments?.find((d) => d.name === query?.deploymentName);

  // The deployment doesn't exist.
  if (
    !isLoading &&
    project &&
    deployments &&
    deployments.length > 0 &&
    !deployment &&
    !!query.deploymentName
  ) {
    void push("/404");
  }

  return deployment;
}

export function useProvisionDeployment(projectId: number) {
  return useBBMutation({
    path: "/projects/{project_id}/provision",
    pathParams: {
      project_id: projectId.toString(),
    },
    mutateKey: `/projects/{project_id}/instances`,
    mutatePathParams: {
      project_id: projectId.toString(),
    },
  });
}

export function useDeploymentById(teamId: number, deploymentId?: number) {
  const { data: deployment } = useBBQuery({
    path: "/teams/{team_id}/deployments/{deployment_id}",
    pathParams: {
      team_id: teamId,
      deployment_id: deploymentId || 0,
    },
  });

  return deployment;
}

export function useDeletePreviewDeployment(projectId?: number) {
  return useBBMutation({
    path: "/projects/{project_id}/delete_preview_deployment",
    pathParams: { project_id: projectId?.toString() || "" },
    mutateKey: `/projects/{project_id}/instances`,
    mutatePathParams: { project_id: projectId?.toString() || "" },
    successToast: "Deleted preview deployment.",
  });
}
