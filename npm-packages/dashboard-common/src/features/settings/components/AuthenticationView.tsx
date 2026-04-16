import { useContext } from "react";
import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { NoPermissionMessage } from "@common/elements/NoPermissionMessage";
import { AuthConfig } from "@common/features/settings/components/AuthConfig";

export function AuthenticationView() {
  const { useIsOperationAllowed } = useContext(DeploymentInfoContext);
  const canViewData = useIsOperationAllowed("ViewData");

  return (
    <DeploymentSettingsLayout page="authentication">
      {!canViewData ? (
        <NoPermissionMessage message="You do not have permission to view authentication config in this deployment." />
      ) : (
        <AuthConfig />
      )}
    </DeploymentSettingsLayout>
  );
}
