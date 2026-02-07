import { ExternalLinkIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { CopyButton } from "@common/elements/CopyButton";
import { ProjectDetails, TeamResponse } from "generatedApi";
import { useDeployments } from "api/deployments";

export function ProjectLink({
  project,
  team,
  memberId,
  isLoading,
}: {
  project: ProjectDetails | null;
  team?: TeamResponse;
  memberId?: number;
  isLoading?: boolean;
}) {
  const { deployments } = useDeployments(project?.id);

  if (isLoading) {
    return <span className="text-content-secondary">Loading...</span>;
  }

  if (!project) {
    return <span className="text-content-secondary">Deleted Project</span>;
  }

  const projectName = project.name;
  const projectSlug = project.slug;
  const showSlug = projectSlug && projectName.toLowerCase() !== projectSlug;

  // Determine which deployment to link to
  const defaultProdDeployment = deployments?.find(
    (d) => d.kind === "cloud" && d.deploymentType === "prod" && d.isDefault,
  );
  const defaultDevDeployment = deployments?.find(
    (d) =>
      d.kind === "cloud" &&
      d.deploymentType === "dev" &&
      d.creator === memberId &&
      d.isDefault,
  );
  const anyDeployment = deployments?.[0];
  const shownDeployment =
    defaultDevDeployment ?? defaultProdDeployment ?? anyDeployment;

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
