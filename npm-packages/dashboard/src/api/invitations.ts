import { useBBMutation, useBBQuery } from "./api";

export function useTeamInvites(teamId: number) {
  const { data: invites } = useBBQuery(`/teams/{team_id}/invites`, {
    team_id: teamId.toString(),
  });

  return invites;
}

export function useCreateInvite(teamId: number) {
  return useBBMutation({
    path: "/teams/{team_id}/invites",
    pathParams: { team_id: teamId.toString() },
    mutateKey: "/teams/{team_id}/invites",
    mutatePathParams: { team_id: teamId.toString() },
    successToast: "Invitation sent.",
  });
}

export function useCancelInvite(teamId: number) {
  return useBBMutation({
    path: "/teams/{team_id}/invites/cancel",
    pathParams: { team_id: teamId.toString() },
    mutateKey: "/teams/{team_id}/invites",
    mutatePathParams: { team_id: teamId.toString() },
    successToast: "Invitation revoked.",
  });
}

export function useAcceptInvite(code: string) {
  return useBBMutation({
    path: "/invites/{code}/accept",
    pathParams: { code },
    toastOnError: false,
  });
}
