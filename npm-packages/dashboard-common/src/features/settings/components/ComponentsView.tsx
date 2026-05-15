import { useContext } from "react";
import { LoadingTransition } from "@ui/Loading";
import { DeploymentSettingsLayout } from "@common/layouts/DeploymentSettingsLayout";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { useNents } from "@common/lib/useNents";
import { Components } from "@common/features/settings/components/Components";
import { NoPermissionMessage } from "@common/elements/NoPermissionMessage";

export function ComponentsView() {
  const {
    useCurrentDeployment,
    useIsOperationAllowed,
    useCustomRolePermission,
    useHasProjectAdminPermissions,
  } = useContext(DeploymentInfoContext);
  const deployment = useCurrentDeployment();
  const isAdmin = useHasProjectAdminPermissions(deployment?.projectId);
  const canViewData = useIsOperationAllowed("ViewData");
  const canViewDataCustom = useCustomRolePermission(
    "deployment:data:view",
    true,
  );
  const canView = isAdmin || canViewDataCustom;
  const { nents } = useNents();

  const body =
    canView === false ? (
      <NoPermissionMessage
        message="You do not have permission to view components in this deployment."
        missingPermission="deployment:data:view"
      />
    ) : !canViewData ? (
      <NoPermissionMessage message="You do not have permission to view components in this deployment." />
    ) : (
      <LoadingTransition>
        {nents && <Components nents={nents} />}
      </LoadingTransition>
    );

  return (
    <DeploymentSettingsLayout page="components">
      {body}
    </DeploymentSettingsLayout>
  );
}
