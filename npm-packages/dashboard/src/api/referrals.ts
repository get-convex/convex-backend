import { useBBMutation, useBBQuery } from "./api";

export function useReferralCode(code: string) {
  const { data } = useBBQuery({
    path: "/validate_referral_code",
    pathParams: undefined,
    queryParams: {
      code,
    },
  });

  if (data === undefined) {
    return undefined;
  }

  if (data === "Invalid") {
    return {
      valid: false as const,
    };
  }

  return {
    valid: true as const,
    teamName: data.Valid.teamName,
    exhausted: data.Valid.exhausted,
  };
}

export function useReferralState(teamId?: number) {
  const { data } = useBBQuery({
    path: "/teams/{team_id}/referral_state",
    pathParams: {
      team_id: teamId?.toString() || "",
    },
  });

  return data;
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
