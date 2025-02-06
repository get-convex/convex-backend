import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import Link from "next/link";
import { useContext } from "react";

export function FunctionRunnerDisabledWhilePaused() {
  const { deploymentsURI } = useContext(DeploymentInfoContext);
  return (
    <>
      The function runner is not available while the deployment is paused. To
      resume your deployment, go to{" "}
      <Link
        passHref
        href={`${deploymentsURI}/settings/pause-deployment`}
        className="text-content-link underline hover:underline"
      >
        settings.
      </Link>
    </>
  );
}
