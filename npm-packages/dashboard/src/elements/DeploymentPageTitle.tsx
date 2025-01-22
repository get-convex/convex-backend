import { useCurrentProject } from "api/projects";
import { useCurrentDeployment } from "api/deployments";
import Head from "next/head";

export function DeploymentPageTitle({
  title,
  subtitle,
}: {
  title: string;
  subtitle?: string;
}) {
  const project = useCurrentProject();
  const deployment = useCurrentDeployment();

  return (
    <Head>
      {project && deployment && (
        <title>
          {title} {subtitle && `| ${subtitle} `}|{" "}
          {capitalize(deployment.deploymentType)} | {project.name} | Convex
          Dashboard
        </title>
      )}
    </Head>
  );
}

function capitalize(text: string) {
  return text.charAt(0).toUpperCase() + text.slice(1);
}
