import { useMutation } from "hooks/useMutation";
import { useBBMutation, useBBQuery, useMutate } from "./api";

export type AccessTokenListKind = "deployment" | "project";

export function useTeamAccessTokens(teamId?: number) {
  const { data: accessTokens } = useBBQuery("/teams/{team_id}/access_tokens", {
    team_id: teamId?.toString() || "",
  });

  return accessTokens;
}

export function useInstanceAccessTokens(deploymentName?: string) {
  const { data: accessTokens } = useBBQuery(
    "/instances/{deployment_name}/access_tokens",
    {
      deployment_name: deploymentName || "",
    },
  );

  return accessTokens;
}

export function useProjectAccessTokens(projectId?: number) {
  const { data: accessTokens } = useBBQuery(
    "/projects/{project_id}/access_tokens",
    {
      project_id: projectId?.toString() || "",
    },
  );

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

export type CreateDeploymentAccessTokenRequest = {
  authnToken: string;
  deviceName: string;
  teamId: number;
  deploymentId: number | null;
  projectId: number | null;
  permissions: string[] | null;
};

export function useCreateTeamAccessToken(
  params:
    | { kind: "deployment"; deploymentName: string }
    | { kind: "project"; projectId: number },
) {
  const mutate = useMutate();

  // We need to use the old untyped useMutation here because the create access token endpoint
  // is not under the dashboard API router.
  // TODO(ari): Add an additional /api/dashboard route for creating access tokens
  // that uses the same handler
  const fn = useMutation<CreateDeploymentAccessTokenRequest>({
    url: `/api/authorize`,
    successToast: "Access token created.",
  });
  return async (args: CreateDeploymentAccessTokenRequest) => {
    const ret = await fn(args);
    params.kind === "deployment"
      ? await mutate(
          [
            "/instances/{deployment_name}/access_tokens",
            {
              params: {
                path: {
                  deployment_name: params.deploymentName,
                },
              },
            },
          ],
          undefined,
        )
      : await mutate(
          [
            "/projects/{project_id}/access_tokens",
            {
              params: {
                path: {
                  project_id: params.projectId.toString(),
                },
              },
            },
          ],
          undefined,
        );
    return ret;
  };
}
