import { useBBMutation, useBBQuery } from "./api";

export function useReferralState(teamId?: number) {
  return useBBQuery({
    path: "/teams/{team_id}/referral_state",
    pathParams: {
      team_id: teamId?.toString() || "",
    },
  });
}

export function useApplyReferralCode(teamId?: number) {
  return useBBMutation({
    path: "/teams/{team_id}/apply_referral_code",
    pathParams: {
      team_id: teamId?.toString() || "",
    },
    successToast: "Congrats! Your referral code has been applied successfully.",
  });
}
