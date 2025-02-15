import { useBBMutation, useBBQuery } from "./api";

export type AccessTokenListKind = "deployment" | "project";

export function useTeamAccessTokens(teamId?: number) {
  const { data: accessTokens } = useBBQuery({
    path: "/teams/{team_id}/access_tokens",
    pathParams: {
      team_id: teamId?.toString() || "",
    },
  });

  return accessTokens;
}

export function useInstanceAccessTokens(deploymentName?: string) {
  const { data: accessTokens } = useBBQuery({
    path: "/instances/{deployment_name}/access_tokens",
    pathParams: {
      deployment_name: deploymentName || "",
    },
  });

  return accessTokens;
}

export function useProjectAccessTokens(projectId?: number) {
  const { data: accessTokens } = useBBQuery({
    path: "/projects/{project_id}/access_tokens",
    pathParams: {
      project_id: projectId?.toString() || "",
    },
  });

  return accessTokens;
}

export function useDeleteAccessToken(
  identifier: string,
  kind: AccessTokenListKind,
) {
  return useBBMutation({
    path: "/teams/delete_access_token",
    pathParams: undefined,
    mutateKey:
      kind === "deployment"
        ? "/instances/{deployment_name}/access_tokens"
        : "/projects/{project_id}/access_tokens",
    mutatePathParams:
      kind === "deployment"
        ? { deployment_name: identifier }
        : { project_id: identifier },
    successToast: "Access token deleted.",
  });
}

export function useDeleteTeamAccessToken(teamId: number) {
  return useBBMutation({
    path: "/teams/delete_access_token",
    pathParams: undefined,
    mutateKey: "/teams/{team_id}/access_tokens",
    mutatePathParams: { team_id: teamId.toString() },
    successToast: "Access token deleted.",
  });
}

export function useCreateTeamAccessToken(
  params:
    | { kind: "deployment"; deploymentName: string }
    | { kind: "project"; projectId: number },
) {
  return useBBMutation({
    path: "/authorize",
    pathParams: undefined,
    mutateKey:
      params.kind === "deployment"
        ? "/instances/{deployment_name}/access_tokens"
        : "/projects/{project_id}/access_tokens",
    mutatePathParams:
      params.kind === "deployment"
        ? {
            deployment_name: params.deploymentName,
          }
        : {
            project_id: params.projectId.toString(),
          },
    successToast: "Access token created.",
  });
}
