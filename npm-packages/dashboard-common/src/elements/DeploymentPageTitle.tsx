import Head from "next/head";
import { useContext } from "react";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";

export function DeploymentPageTitle({
  title,
  subtitle,
}: {
  title: string;
  subtitle?: string;
}) {
  const { useCurrentProject, useCurrentDeployment } = useContext(
    DeploymentInfoContext,
  );
  const project = useCurrentProject();
  const deployment = useCurrentDeployment();
  const deploymentId = deployment && "id" in deployment ? deployment.id : null;

  return (
    <Head>
      {project && deployment && (
        <title>
          {title} {subtitle && `| ${subtitle} `}
          {deploymentId === 0
            ? null
            : `| ${capitalize(deployment.deploymentType)}`}{" "}
          {project.id === 0 ? null : `| ${project.name}`} | Convex Dashboard
        </title>
      )}
    </Head>
  );
}

function capitalize(text: string) {
  return text.charAt(0).toUpperCase() + text.slice(1);
}
