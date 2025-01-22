import { useBBMutation, useBBQuery } from "./api";

export function useListVanityDomains(deploymentName?: string) {
  const { data } = useBBQuery(
    "/instances/{deployment_name}/domains/list",
    {
      deployment_name: deploymentName || "",
    },
    {
      refreshInterval: 5000,
    },
  );
  return data?.domains;
}

export function useCreateVanityDomain(deploymentName: string) {
  return useBBMutation({
    path: `/instances/{deployment_name}/domains/create`,
    pathParams: {
      deployment_name: deploymentName,
    },
    mutateKey: `/instances/{deployment_name}/domains/list`,
    mutatePathParams: {
      deployment_name: deploymentName,
    },
    successToast:
      "Custom domain has been added. Your changes may take up to 30 minutes to be propagated.",
  });
}

export function useDeleteVanityDomain(deploymentName: string) {
  return useBBMutation({
    path: "/instances/{deployment_name}/domains/delete",
    pathParams: {
      deployment_name: deploymentName,
    },
    mutateKey: "/instances/{deployment_name}/domains/list",
    mutatePathParams: {
      deployment_name: deploymentName,
    },
    successToast: "Custom domain has been deleted.",
  });
}
