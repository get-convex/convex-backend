import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import { Integrations } from "@common/features/settings/components/integrations/Integrations";
import { useQuery } from "convex/react";
import udfs from "@common/udfs";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { useContext } from "react";
import { LoadingTransition } from "@ui/Loading";

export function IntegrationsView({
  onAddedIntegration,
  showPostHogIntegrations,
}: {
  onAddedIntegration?: (kind: string) => void;
  showPostHogIntegrations?: boolean;
}) {
  const {
    useCurrentTeam,
    useTeamEntitlements,
    useCurrentDeployment,
    workOSOperations,
  } = useContext(DeploymentInfoContext);
  const team = useCurrentTeam();
  const deployment = useCurrentDeployment();
  const entitlements = useTeamEntitlements(team?.id);
  const integrations = useQuery(udfs.listConfiguredSinks.default);
  const workosData = workOSOperations.useDeploymentWorkOSEnvironment(
    deployment?.name,
  );

  return (
    <DeploymentSettingsLayout page="integrations">
      <LoadingTransition>
        {team && entitlements && integrations !== undefined && (
          <Integrations
            team={team}
            entitlements={entitlements}
            integrations={integrations}
            workosData={workosData}
            onAddedIntegration={onAddedIntegration}
            showPostHogIntegrations={showPostHogIntegrations}
          />
        )}
      </LoadingTransition>
    </DeploymentSettingsLayout>
  );
}
