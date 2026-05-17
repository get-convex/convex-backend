import { useContext } from "react";
import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { NoPermissionMessage } from "@common/elements/NoPermissionMessage";
import { AuthConfig } from "@common/features/settings/components/AuthConfig";

export function AuthenticationView() {
  const {
    useCurrentDeployment,
    useIsOperationAllowed,
    useCustomRolePermission,
    useHasProjectAdminPermissions,
  } = useContext(DeploymentInfoContext);
  const deployment = useCurrentDeployment();
  const isAdmin = useHasProjectAdminPermissions(deployment?.projectId);
  const canViewDataOp = useIsOperationAllowed("ViewData");
  const canViewEnvCustom = useCustomRolePermission("deployment:env:view", true);
  const canViewDataCustom = useCustomRolePermission(
    "deployment:data:view",
    true,
  );
  const canViewEnv = isAdmin || canViewEnvCustom;
  // `AuthConfig` reads from `_auth_config`, which is gated server-side
  // on `ViewData`; custom-role members need an explicit
  // `deployment:data:view` grant in addition to `deployment:env:view`.
  const canViewData = isAdmin || canViewDataCustom !== false;

  const body =
    canViewEnv === false ? (
      <NoPermissionMessage
        message="You do not have permission to view authentication config."
        missingPermission="deployment:env:view"
      />
    ) : canViewData === false ? (
      <NoPermissionMessage
        message="You do not have permission to view authentication config."
        missingPermission="deployment:data:view"
      />
    ) : !canViewDataOp ? (
      <NoPermissionMessage message="You do not have permission to view authentication config in this deployment." />
    ) : (
      <AuthConfig />
    );

  return (
    <DeploymentSettingsLayout page="authentication">
      {body}
    </DeploymentSettingsLayout>
  );
}
