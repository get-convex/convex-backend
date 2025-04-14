import { SWRConfiguration } from "swr";

import { ProjectEnvVarConfig } from "@common/features/settings/lib/types";
import { useBBMutation, useBBQuery } from "api/api";

export type ProjectEnvironmentVariable = {
  name: string;
  value: string;
};

export function useProjectEnvironmentVariables(
  projectId?: number,
  refreshInterval?: SWRConfiguration["refreshInterval"],
): { configs: ProjectEnvVarConfig[] } | undefined {
  const { data } = useBBQuery({
    path: `/projects/{project_id}/environment_variables/list`,
    pathParams: {
      project_id: projectId?.toString() || "",
    },
    swrOptions: { refreshInterval },
  });
  return data;
}

export function useUpdateProjectEnvVars(projectId?: number) {
  return useBBMutation({
    path: "/projects/{project_id}/environment_variables/update_batch",
    pathParams: { project_id: projectId?.toString() || "" },
    mutateKey: `/projects/{project_id}/environment_variables/list`,
    mutatePathParams: { project_id: projectId?.toString() || "" },
    successToast: "Environment variables updated.",
  });
}
