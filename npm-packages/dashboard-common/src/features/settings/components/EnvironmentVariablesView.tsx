import { useContext } from "react";
import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import { DeploymentEnvironmentVariables } from "@common/features/settings/components/DeploymentEnvironmentVariables";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { NoPermissionMessage } from "@common/elements/NoPermissionMessage";

export function EnvironmentVariablesView({
  onEnvironmentVariablesAdded,
}: {
  onEnvironmentVariablesAdded?: (count: number) => void;
}) {
  const {
    useCurrentDeployment,
    useCustomRolePermission,
    useHasProjectAdminPermissions,
  } = useContext(DeploymentInfoContext);
  const deployment = useCurrentDeployment();
  const isAdmin = useHasProjectAdminPermissions(deployment?.projectId);
  const canViewCustom = useCustomRolePermission("deployment:env:view", true);
  const canView = isAdmin || canViewCustom;

  return (
    <DeploymentSettingsLayout page="environment-variables">
      {canView === false ? (
        <NoPermissionMessage
          message="You do not have permission to view environment variables."
          missingPermission="deployment:env:view"
        />
      ) : (
        <DeploymentEnvironmentVariables
          onEnvironmentVariablesAdded={onEnvironmentVariablesAdded}
        />
      )}
    </DeploymentSettingsLayout>
  );
}
