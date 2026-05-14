import { useContext } from "react";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";

/**
 * Whether the current member is allowed to call deployment system UDFs
 * (`udfs.convexCloudUrl`, `udfs.getVersion`, `udfs.deploymentState`,
 * `udfs.getSchemas`, etc.). Mirrors the server-side `ViewData` gate plus
 * the custom-role `deployment:data:view` permission, with a project
 * admin override.
 *
 * Use this to skip `useQuery(udf, ...)` calls when the member is denied
 * — otherwise the query throws and the error escapes the React tree via
 * the next WebSocket transition (uncatchable by an ErrorBoundary).
 *
 * Returns `true` while the role list is loading so the query fires
 * eagerly; a server-side reject is still possible if the role hasn't
 * propagated to the deployment yet.
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
