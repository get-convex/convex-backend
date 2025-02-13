import { useBBQuery } from "api/api";

export type UsageState =
  | "Default"
  | "Approaching"
  | "Exceeded"
  | "Disabled"
  | "Paused";

export function useTeamUsageState(
  teamId: number | null,
): UsageState | undefined {
  const { data } = useBBQuery(
    "/teams/{team_id}/usage/team_usage_state",
    {
      team_id: teamId?.toString() || "",
    },
    { refreshInterval: 0 },
  );
  return data?.usageState;
}

export function useCurrentBillingPeriod(teamId: number) {
  const { data } = useBBQuery("/teams/{team_id}/usage/current_billing_period", {
    team_id: teamId.toString(),
  });
  return data;
}
