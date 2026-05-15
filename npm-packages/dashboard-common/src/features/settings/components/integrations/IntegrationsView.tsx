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
}: {
  onAddedIntegration?: (kind: string) => void;
}) {
  const {
    useCurrentTeam,
    useTeamEntitlements,
    useCurrentDeployment,
    useIsOperationAllowed,
    useCustomRolePermission,
    useHasProjectAdminPermissions,
    workOSOperations,
  } = useContext(DeploymentInfoContext);
  const team = useCurrentTeam();
  const deployment = useCurrentDeployment();
  const entitlements = useTeamEntitlements(team?.id);
  const isAdmin = useHasProjectAdminPermissions(deployment?.projectId);
  const canViewIntegrations = useIsOperationAllowed("ViewIntegrations");
  const canViewIntegrationsCustomRaw = useCustomRolePermission(
    "deployment:integrations:view",
    true,
  );
  const canViewIntegrationsCustom = isAdmin || canViewIntegrationsCustomRaw;
  const integrations = useQuery(
    udfs.listConfiguredSinks.default,
    canViewIntegrations && canViewIntegrationsCustom !== false
      ? undefined
      : "skip",
  );
  const { data: workosData } = workOSOperations.useDeploymentWorkOSEnvironment(
    deployment?.name,
  );

  const body =
    canViewIntegrationsCustom === false ? (
      <NoPermissionMessage
        message="You do not have permission to view integrations."
        missingPermission="deployment:integrations:view"
      />
    ) : !canViewIntegrations ? (
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
          />
        )}
      </LoadingTransition>
    );

  return (
    <DeploymentSettingsLayout page="integrations">
      {body}
    </DeploymentSettingsLayout>
  );
}
