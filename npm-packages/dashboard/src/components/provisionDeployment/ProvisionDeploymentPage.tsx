import { useCurrentTeam } from "api/teams";
import { useCurrentProject } from "api/projects";
import { useRouter } from "next/router";
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

  if (projectId === null) {
    return null;
  }

  return (
    <ProvisionDeploymentForm
      projectId={projectId}
      projectURI={projectURI}
      deploymentType={deploymentType}
    />
  );
}
