import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { useContext, useMemo } from "react";
import { useSessionStorage } from "react-use";

/**
 * Some deployments (e.g. production deployments by default) require the user to
 * unlock edits before they can make changes to the data.
 *
 * This hook is used to determine whether this authorization is necessary.
 * It is often used in conjunction with `<AuthorizeEditsConfirmationDialog />`.
 */
export function useEditsAuthorization() {
  const { useIsProtectedDeployment } = useContext(DeploymentInfoContext);
  const isProtectedDeployment = useIsProtectedDeployment();

  const [protectedEditsEnabled, setProtectedEditsEnabled] = useSessionStorage(
    "protectedEditsEnabled",
    false,
  );
  const authorizeEdits = useMemo(
    () =>
      isProtectedDeployment ? () => setProtectedEditsEnabled(true) : undefined,
    [isProtectedDeployment, setProtectedEditsEnabled],
  );
  const areEditsAuthorized = !isProtectedDeployment || protectedEditsEnabled;
  return { areEditsAuthorized, authorizeEdits };
}
