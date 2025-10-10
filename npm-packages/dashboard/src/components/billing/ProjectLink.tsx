import { ExternalLinkIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { CopyButton } from "@common/elements/CopyButton";
import { ProjectDetails, Team } from "generatedApi";
import { useDeployments } from "api/deployments";

export function ProjectLink({
  project,
  team,
  memberId,
}: {
  project: ProjectDetails | null;
  team?: Team;
  memberId?: number;
}) {
  const { deployments } = useDeployments(project?.id);

  if (!project) {
    return <span>Deleted Project</span>;
  }

  const projectName = project.name;
  const projectSlug = project.slug;
  const showSlug = projectSlug && projectName.toLowerCase() !== projectSlug;

  // Determine which deployment to link to
  const prodDeployment = deployments?.find((d) => d.deploymentType === "prod");
  const devDeployment = deployments?.find(
    (d) => d.deploymentType === "dev" && d.creator === memberId,
  );
  const anyDeployment = deployments?.[0];
  const shownDeployment = devDeployment ?? prodDeployment ?? anyDeployment;

  const href =
    team && shownDeployment
      ? `/t/${team.slug}/${projectSlug}/${shownDeployment.name}`
      : undefined;

  return (
    <span className="flex items-center gap-1">
      <span className="text-sm font-medium text-content-primary">
        {projectName}
        {showSlug && (
          <span className="text-content-secondary"> ({projectSlug})</span>
        )}
      </span>
      {href && (
        <Button
          href={href}
          inline
          variant="neutral"
          icon={<ExternalLinkIcon />}
          tip="Open project in new tab"
          target="_blank"
          disabled={!shownDeployment}
          aria-label="Open project"
        />
      )}
      {projectSlug && (
        <CopyButton text={projectSlug} inline tip="Copy project slug" />
      )}
    </span>
  );
}
