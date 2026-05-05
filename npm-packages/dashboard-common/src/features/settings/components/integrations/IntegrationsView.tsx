import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import { NoPermissionMessage } from "@common/elements/NoPermissionMessage";
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
    useIsOperationAllowed,
    workOSOperations,
  } = useContext(DeploymentInfoContext);
  const team = useCurrentTeam();
  const deployment = useCurrentDeployment();
  const entitlements = useTeamEntitlements(team?.id);
  const canViewIntegrations = useIsOperationAllowed("ViewIntegrations");
  const integrations = useQuery(
    udfs.listConfiguredSinks.default,
    canViewIntegrations ? undefined : "skip",
  );
  const { data: workosData } = workOSOperations.useDeploymentWorkOSEnvironment(
    deployment?.name,
  );

  return (
    <DeploymentSettingsLayout page="integrations">
      {!canViewIntegrations ? (
        <NoPermissionMessage message="You do not have permission to view integrations in this deployment." />
      ) : (
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
      )}
    </DeploymentSettingsLayout>
  );
}
