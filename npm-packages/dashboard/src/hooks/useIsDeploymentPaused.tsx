import udfs from "@common/udfs";
import { useQuery } from "convex/react";
import { useCurrentTeam } from "api/teams";
import { useTeamUsageState } from "api/usage";
import { useCanViewDeploymentData } from "@common/lib/useCanViewDeploymentData";

export function useIsDeploymentPaused() {
  const currentTeam = useCurrentTeam();
  const teamState = useTeamUsageState(currentTeam?.id ?? null);
  // Skip the system UDF when the member can't view data — otherwise it
  // throws and the error escapes React's tree on the next WebSocket
  // notify.
  const canViewData = useCanViewDeploymentData();
  const deploymentState = useQuery(
    udfs.deploymentState.deploymentState,
    canViewData ? {} : "skip",
  );

  if (
    currentTeam === undefined ||
    teamState === undefined ||
    (canViewData && deploymentState === undefined)
  ) {
    return undefined;
  }

  return (
    teamState === "Paused" ||
    teamState === "Disabled" ||
    deploymentState?.state === "paused"
  );
}
