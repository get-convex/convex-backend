import { useBBMutation, useBBQuery } from "./api";

export function useTeamOauthApps(teamId?: number) {
  return useBBQuery({
    path: "/teams/{team_id}/oauth_apps",
    pathParams: { team_id: teamId?.toString() || "" },
  });
}

export function useRegisterOauthApp(teamId: number) {
  return useBBMutation({
    path: "/teams/{team_id}/oauth_apps/register",
    pathParams: { team_id: teamId.toString() },
    successToast: "OAuth app registered.",
    mutateKey: "/teams/{team_id}/oauth_apps",
    mutatePathParams: { team_id: teamId.toString() },
    toastOnError: false,
  });
}

export function useUpdateOauthApp(teamId: number, clientId: string) {
  return useBBMutation({
    path: "/teams/{team_id}/oauth_apps/{client_id}/update",
    pathParams: { team_id: teamId, client_id: clientId },
    successToast: "OAuth app updated.",
    mutateKey: "/teams/{team_id}/oauth_apps",
    mutatePathParams: { team_id: teamId.toString() },
    toastOnError: false,
  });
}

export function useDeleteOauthApp(teamId: number, clientId: string) {
  return useBBMutation({
    path: "/teams/{team_id}/oauth_apps/{client_id}/delete",
    pathParams: { team_id: teamId, client_id: clientId },
    successToast: "OAuth app deleted.",
    mutateKey: "/teams/{team_id}/oauth_apps",
    mutatePathParams: { team_id: teamId.toString() },
  });
}

export function useCheckOauthApp(teamId?: number) {
  return useBBMutation({
    path: "/teams/{team_id}/oauth_apps/check",
    pathParams: { team_id: teamId?.toString() || "" },
  });
}
