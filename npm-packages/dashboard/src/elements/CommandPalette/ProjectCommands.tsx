import { Command } from "cmdk";
import React, { useMemo } from "react";
import { useCurrentTeam } from "api/teams";
import { useDeployments } from "api/deployments";
import type {
  PlatformDeploymentResponse,
  ProjectDetails,
  TeamResponse,
} from "generatedApi";
import { projectNavigation, projectSectionNavigation } from "./navigation";
import { DeploymentItem, LoadingSignal, NavigationItem } from "./items";

// Commands for a drilled-into project: its pages plus all of its (cloud)
// deployments, which drill further into deployment pages.
export function ProjectCommands({
  project,
  onNavigate,
  onSelectDeployment,
}: {
  project: ProjectDetails;
  onNavigate: (href: string) => void;
  onSelectDeployment: (deployment: PlatformDeploymentResponse) => void;
}) {
  const team = useCurrentTeam();

  if (!team) {
    return <LoadingSignal />;
  }

  return (
    <>
      <Command.Group heading="Project">
        {[
          ...projectNavigation(team.slug, project.slug, project.name),
          ...projectSectionNavigation(team.slug, project.slug),
        ].map((target) => (
          <NavigationItem
            key={target.label}
            target={target}
            onNavigate={onNavigate}
          />
        ))}
      </Command.Group>
      <DeploymentsGroup
        team={team}
        project={project}
        onNavigate={onNavigate}
        onSelectDeployment={onSelectDeployment}
      />
    </>
  );
}

// The drilled-into "Switch Deployment" page: just the project's deployments,
// without the project's own pages.
export function SwitchDeploymentCommands({
  project,
  onNavigate,
  onSelectDeployment,
}: {
  project: ProjectDetails;
  onNavigate: (href: string) => void;
  onSelectDeployment: (deployment: PlatformDeploymentResponse) => void;
}) {
  const team = useCurrentTeam();

  if (!team) {
    return <LoadingSignal />;
  }

  return (
    <DeploymentsGroup
      team={team}
      project={project}
      onNavigate={onNavigate}
      onSelectDeployment={onSelectDeployment}
    />
  );
}

function DeploymentsGroup({
  team,
  project,
  onNavigate,
  onSelectDeployment,
}: {
  team: TeamResponse;
  project: ProjectDetails;
  onNavigate: (href: string) => void;
  onSelectDeployment: (deployment: PlatformDeploymentResponse) => void;
}) {
  const { deployments, isLoading } = useDeployments(project.id);
  // Local deployments have no dashboard pages to navigate to.
  const cloudDeployments = useMemo(
    () => (deployments ?? []).filter((d) => d.kind === "cloud"),
    [deployments],
  );

  return (
    <Command.Group heading="Deployments">
      {isLoading && !deployments ? (
        <LoadingSignal />
      ) : (
        cloudDeployments.map((deployment) => (
          <DeploymentItem
            key={deployment.name}
            deployment={deployment}
            teamSlug={team.slug}
            projectSlug={project.slug}
            onNavigate={onNavigate}
            onDrill={() => onSelectDeployment(deployment)}
          />
        ))
      )}
    </Command.Group>
  );
}
