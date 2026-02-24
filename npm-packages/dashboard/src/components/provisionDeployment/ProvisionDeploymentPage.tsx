import { useCurrentTeam } from "api/teams";
import { useCurrentProject } from "api/projects";
import { useDeployments } from "api/deployments";
import { useProfile } from "api/profile";
import { useRouter } from "next/router";
import { useEffect, useMemo } from "react";
import { Loading } from "@ui/Loading";
import { ProvisionDeploymentForm } from "./ProvisionDeploymentForm";

export function ProvisionDeploymentPage({
  deploymentType,
}: {
  deploymentType: "prod" | "dev";
}) {
  const router = useRouter();
  const projectSlug = router.query.project as string;
  const team = useCurrentTeam();
  const project = useCurrentProject();
  const projectId = project?.id ?? null;
  const projectURI = `/t/${team?.slug}/${projectSlug}`;

  const { deployments } = useDeployments(projectId ?? undefined);
  const member = useProfile();

  const existingDeployment = useMemo(() => {
    if (!deployments) return undefined;
    if (deploymentType === "prod") {
      return (
        deployments.find(
          (d) =>
            d.kind === "cloud" && d.deploymentType === "prod" && d.isDefault,
        ) ?? null
      );
    }
    if (member === undefined) return undefined;
    return (
      deployments.find(
        (d) =>
          d.kind === "cloud" &&
          d.deploymentType === "dev" &&
          d.isDefault &&
          d.creator === member?.id,
      ) ?? null
    );
  }, [deployments, deploymentType, member]);

  useEffect(() => {
    if (existingDeployment) {
      void router.replace(`${projectURI}/${existingDeployment.name}`);
    }
  }, [existingDeployment, projectURI, router]);

  if (
    projectId === null ||
    deployments === undefined ||
    existingDeployment === undefined
  ) {
    return <Loading />;
  }

  return (
    <ProvisionDeploymentForm
      projectId={projectId}
      projectURI={projectURI}
      deploymentType={deploymentType}
    />
  );
}
