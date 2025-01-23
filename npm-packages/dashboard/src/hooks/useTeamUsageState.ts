import useSWR from "swr";

export type UsageState =
  | "Default"
  | "Approaching"
  | "Exceeded"
  | "Disabled"
  | "Paused";

export function useTeamUsageState(
  teamId: number | null,
): UsageState | undefined {
  const { data } = useSWR<{
    usageState: UsageState;
  }>(
    teamId
      ? `/api/dashboard/teams/${teamId}/usage/team_usage_state`
      : undefined,
  );

  if (data === undefined) {
    return undefined;
  }

  return data.usageState;
}
