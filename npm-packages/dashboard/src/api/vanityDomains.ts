import { useManagementApiMutation, useManagementApiQuery } from "./api";

export function useListVanityDomains(deploymentName?: string) {
  const { data } = useManagementApiQuery({
    path: "/deployments/{deployment_name}/custom_domains",
    pathParams: {
      deployment_name: deploymentName || "",
    },
    swrOptions: {
      refreshInterval: 5000,
    },
  });
  return data?.domains;
}

export function useCreateVanityDomain(deploymentName: string) {
  return useManagementApiMutation({
    path: `/deployments/{deployment_name}/create_custom_domain`,
    pathParams: {
      deployment_name: deploymentName,
    },
    mutateKey: `/deployments/{deployment_name}/custom_domains`,
    mutatePathParams: {
      deployment_name: deploymentName,
    },
    successToast:
      "Custom domain has been added. Your changes may take up to 30 minutes to be propagated.",
  });
}

export function useDeleteVanityDomain(deploymentName: string) {
  return useManagementApiMutation({
    path: "/deployments/{deployment_name}/delete_custom_domain",
    pathParams: {
      deployment_name: deploymentName,
    },
    mutateKey: "/deployments/{deployment_name}/custom_domains",
    mutatePathParams: {
      deployment_name: deploymentName,
    },
    successToast: "Custom domain has been deleted.",
  });
}
