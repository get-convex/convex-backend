import udfs from "@common/udfs";
import { useQuery } from "convex/react";
import { useCurrentTeam } from "api/teams";
import { useTeamUsageState } from "api/usage";

export function useIsDeploymentPaused() {
  const currentTeam = useCurrentTeam();
  const teamState = useTeamUsageState(currentTeam?.id ?? null);
  const deploymentState = useQuery(udfs.deploymentState.deploymentState);

  if (
    currentTeam === undefined ||
    teamState === undefined ||
    deploymentState === undefined
  ) {
    return undefined;
  }

  return (
    teamState === "Paused" ||
    teamState === "Disabled" ||
    deploymentState.state === "paused"
  );
}
