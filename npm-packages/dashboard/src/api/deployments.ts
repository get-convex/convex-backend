import { useRouter } from "next/router";
import { useMemo } from "react";
import { useInitialData } from "hooks/useServerSideData";
import { useProfile } from "./profile";
import { useCurrentProject } from "./projects";
import { useManagementApiMutation, useManagementApiQuery } from "./api";
import { useDeploymentsPageSize } from "hooks/useDeploymentsPageSize";

export function useDeployments(projectId?: number) {
  const [initialData] = useInitialData();
  const { data, isLoading } = useManagementApiQuery({
    path: "/projects/{project_id}/list_deployments",
    pathParams: {
      project_id: projectId || 0,
    },
    queryParams: {
      includeLocal: true,
    },
    swrOptions: {
      revalidateOnMount: initialData === undefined,
      refreshInterval: 30 * 1000,
      keepPreviousData: false,
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
      d.creator === member?.id &&
      d.isDefault,
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
  const deploymentName =
    typeof query.deploymentName === "string" ? query.deploymentName : undefined;
  const deployment = deployments?.find((d) => d.name === deploymentName);
  const projectSlug =
    typeof query.project === "string" ? query.project : undefined;

  // The deployment doesn't exist.
  if (
    !isLoading &&
    project &&
    deployments &&
    deployments.length > 0 &&
    !deployment &&
    deploymentName
  ) {
    if (projectSlug && typeof window !== "undefined") {
      const key = `/lastViewedDeploymentForProject/${projectSlug}`;
      const lastViewedDeploymentForProject = window.localStorage.getItem(key);
      if (lastViewedDeploymentForProject === deploymentName) {
        window.localStorage.removeItem(key);
      }
    }
    void push("/404");
  }

  return deployment;
}

export function useProvisionDeployment(projectId: number) {
  return useManagementApiMutation({
    path: "/projects/{project_id}/create_deployment",
    pathParams: {
      project_id: projectId,
    },
    mutateKey: `/projects/{project_id}/list_deployments`,
    mutatePathParams: {
      project_id: projectId,
    },
  });
}

export function useModifyDeploymentSettings({
  deploymentName,
  projectId,
}: {
  deploymentName: string | undefined;
  projectId: number | undefined;
}) {
  return useManagementApiMutation({
    path: "/deployments/{deployment_name}",
    pathParams: {
      deployment_name: deploymentName ?? "",
    },
    method: "patch",
    mutateKey: `/projects/{project_id}/list_deployments`,
    mutatePathParams: {
      project_id: projectId ?? 0,
    },
    successToast: "Deployment settings updated successfully",
  });
}

export function useDeploymentRegions(teamId: number | undefined) {
  const { data, isLoading } = useManagementApiQuery({
    path: "/teams/{team_id}/list_deployment_regions",
    pathParams: {
      team_id: teamId?.toString() || "",
    },
  });

  return { regions: data?.items, isLoading };
}

export function useDeploymentByName(deploymentName?: string) {
  const { data: deployment } = useManagementApiQuery({
    path: "/deployments/{deployment_name}",
    pathParams: {
      deployment_name: deploymentName || "",
    },
  });

  return deployment;
}

export function useDeleteDeployment(
  projectId: number,
  deploymentName: string,
  settingsUrl: string,
) {
  const deleteDeployment = useManagementApiMutation({
    path: "/deployments/{deployment_name}/delete",
    method: "post",
    pathParams: { deployment_name: deploymentName || "" },
    mutateKey: `/projects/{project_id}/list_deployments`,
    mutatePathParams: { project_id: projectId },
    successToast: "Deleted deployment.",
    redirectTo: settingsUrl,
  });

  return deleteDeployment;
}

export function useTransferDeployment(deploymentName: string) {
  return useManagementApiMutation({
    path: "/deployments/{deployment_name}/transfer",
    method: "post",
    pathParams: { deployment_name: deploymentName },
    successToast: "Deployment transferred.",
  });
}

export function usePaginatedDeployments(
  teamId: number | undefined,
  options: {
    cursor?: string;
    sortBy?: string;
    sortOrder?: string;
    deploymentType?: string;
    q?: string;
    projectId?: number;
    creator?: number;
    isDefault?: boolean;
  },
  refreshInterval?: number,
) {
  const { pageSize } = useDeploymentsPageSize();

  const {
    cursor,
    sortBy,
    sortOrder,
    deploymentType,
    q,
    projectId,
    creator,
    isDefault,
  } = options;

  // Note: the OpenAPI spec uses snake_case names, but the Rust handler has
  // #[serde(rename_all = "camelCase")] so it actually expects camelCase params.
  const queryParams = useMemo(
    () => ({
      cursor,
      limit: pageSize,
      sortBy,
      sortOrder,
      deploymentType,
      q,
      projectId,
      creator,
      isDefault,
    }),
    [
      cursor,
      pageSize,
      sortBy,
      sortOrder,
      deploymentType,
      q,
      projectId,
      creator,
      isDefault,
    ],
  );

  const { data, isLoading } = useManagementApiQuery({
    path: "/teams/{team_id}/list_deployments",
    pathParams: {
      team_id: teamId ?? 0,
    },
    queryParams,
    swrOptions: { refreshInterval },
  });

  if (data === undefined) {
    return undefined;
  }

  return { ...data, isLoading };
}
