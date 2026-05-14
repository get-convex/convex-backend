import { useContext } from "react";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";

/**
 * Whether the current member is allowed to call deployment system UDFs
 * (`udfs.convexCloudUrl`, `udfs.getVersion`, `udfs.deploymentState`,
 * `udfs.getSchemas`, etc.). Mirrors the server-side `ViewData` gate plus
 * the custom-role `deployment:data:view` permission, with a project
 * admin override.
 */
export function useCanViewDeploymentData(): boolean {
  const {
    useCurrentDeployment,
    useHasProjectAdminPermissions,
    useIsOperationAllowed,
    useCustomRolePermission,
  } = useContext(DeploymentInfoContext);
  const deployment = useCurrentDeployment();
  const hasAdminPermissions = useHasProjectAdminPermissions(
    deployment?.projectId,
  );
  const canViewDataOp = useIsOperationAllowed("ViewData");
  const canViewDataCustom = useCustomRolePermission(
    "deployment:data:view",
    true,
  );
  return canViewDataOp && (hasAdminPermissions || canViewDataCustom !== false);
}
