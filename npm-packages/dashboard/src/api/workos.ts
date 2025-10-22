import { useBBQuery } from "./api";

export function useDeploymentWorkOSEnvironment(deploymentName?: string) {
  const { data } = useBBQuery({
    path: "/deployments/{deployment_name}/workos_environment",
    pathParams: { deployment_name: deploymentName || "" },
  });
  return data;
}
