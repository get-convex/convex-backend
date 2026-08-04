import { Command } from "cmdk";
import React from "react";
import { useCurrentTeam } from "api/teams";
import { useProjectById } from "api/projects";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";
import type { PlatformDeploymentResponse } from "generatedApi";
import {
  deploymentNavigation,
  deploymentSectionNavigation,
} from "./navigation";
import { LoadingSignal, NavigationItem } from "./items";

export function DeploymentCommands({
  deployment,
  projectSlug: knownProjectSlug,
  onNavigate,
}: {
  deployment: PlatformDeploymentResponse;
  projectSlug?: string;
  onNavigate: (href: string) => void;
}) {
  const team = useCurrentTeam();
  const { project } = useProjectById(deployment.projectId);
  const { usageLimits } = useLaunchDarkly();
  const projectSlug = knownProjectSlug ?? project?.slug;

  if (!team || !projectSlug) {
    return <LoadingSignal />;
  }

  const uriPrefix = `/t/${team.slug}/${projectSlug}/${deployment.name}`;
  // The deployment context lives in the breadcrumb, so the pages don't repeat
  // it on a second line.
  const { pages, settings } = deploymentNavigation(uriPrefix, {
    usageLimitsEnabled: usageLimits,
  });

  return (
    <>
      <Command.Group heading="Pages">
        {pages.map((target) => (
          <NavigationItem
            key={target.label}
            target={target}
            onNavigate={onNavigate}
          />
        ))}
      </Command.Group>
      <Command.Group heading="Settings">
        {[...settings, ...deploymentSectionNavigation(uriPrefix)].map(
          (target) => (
            <NavigationItem
              key={target.label}
              target={target}
              onNavigate={onNavigate}
            />
          ),
        )}
      </Command.Group>
    </>
  );
}
