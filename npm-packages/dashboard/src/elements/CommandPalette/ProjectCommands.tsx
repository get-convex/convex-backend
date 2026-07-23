import { Command } from "cmdk";
import React, { useMemo } from "react";
import { useCurrentTeam } from "api/teams";
import { useDeployments } from "api/deployments";
import type { PlatformDeploymentResponse, ProjectDetails } from "generatedApi";
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
  const { deployments, isLoading } = useDeployments(project.id);
  // Local deployments have no dashboard pages to navigate to.
  const cloudDeployments = useMemo(
    () => (deployments ?? []).filter((d) => d.kind === "cloud"),
    [deployments],
  );

  if (!team) {
    return <LoadingSignal />;
  }

  return (
    <>
      <Command.Group heading={`Project · ${project.name || project.slug}`}>
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
    </>
  );
}
