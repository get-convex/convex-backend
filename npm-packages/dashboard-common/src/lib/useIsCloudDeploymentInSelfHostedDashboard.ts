import { useContext } from "react";
import { DeploymentInfoContext } from "./deploymentContext";

/**
 * Determines if the deployment URL is a default cloud deployment URL.
 *
 * This gives a false negative if the deployment is a cloud deployment with a custom domain.
 */
export function useIsCloudDeploymentInSelfHostedDashboard():
  | {
      isCloudDeploymentInSelfHostedDashboard: false;
      deploymentName: undefined;
    }
  | {
      isCloudDeploymentInSelfHostedDashboard: true;
      deploymentName: string;
    } {
  const context = useContext(DeploymentInfoContext);

  if (
    !context.isSelfHosted ||
    !("deploymentUrl" in context) ||
    !context.deploymentUrl
  ) {
    return {
      isCloudDeploymentInSelfHostedDashboard: false,
      deploymentName: undefined,
    };
  }

  const match = context.deploymentUrl.match(
    /^https:\/\/([a-z]+-[a-z]+-[0-9]{3})\.convex\.cloud$/,
  );

  if (!match) {
    return {
      isCloudDeploymentInSelfHostedDashboard: false,
      deploymentName: undefined,
    };
  }

  return {
    isCloudDeploymentInSelfHostedDashboard: true,
    deploymentName: match[1],
  };
}
