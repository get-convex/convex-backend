import { useDefaultDevDeployment } from "api/deployments";
import { useTeamMembers, useTeamEntitlements } from "api/teams";
import { useProfile } from "api/profile";
import { useRouter } from "next/router";
import React, { useRef } from "react";
import { CommandLineIcon, SignalIcon } from "@heroicons/react/24/outline";
import { Tooltip } from "@ui/Tooltip";
import {
  GearIcon,
  GlobeIcon,
  Pencil2Icon,
  Share1Icon,
} from "@radix-ui/react-icons";
import { DeploymentResponse, ProjectDetails, Team } from "generatedApi";
import { PROVISION_PROD_PAGE_NAME } from "@common/lib/deploymentContext";
import { useIsOverflowing } from "@common/lib/useIsOverflowing";
import { ContextMenu } from "@common/features/data/components/ContextMenu";
import { Key } from "@ui/KeyboardShortcut";

export function DeploymentMenuOptions({
  team,
  project,
  deployments,
}: {
  team: Team;
  project: ProjectDetails;
  deployments: DeploymentResponse[];
}) {
  const member = useProfile();
  const router = useRouter();
  const arePreviewDeploymentsAvailable =
    useTeamEntitlements(team.id)?.projectMaxPreviewDeployments !== 0;

  const previews = deployments
    .filter((d) => d.deploymentType === "preview")
    .sort((a, b) => b.createTime - a.createTime);

  const members = useTeamMembers(team.id);
  const teamMemberDeployments = deployments
    .filter(
      (d) =>
        d.kind === "cloud" &&
        d.deploymentType === "dev" &&
        d.creator !== member?.id,
    )
    .map((d) => {
      const whose = members?.find((tm) => tm.id === d.creator);
      return {
        name: d.name,
        creator: whose?.name || whose?.email || "Teammate",
      };
    })
    .sort((a, b) => a.creator.localeCompare(b.creator));

  const projectSlug = project.slug;

  const selectedTeamSlug = team.slug;

  const prod = deployments?.find((d) => d.deploymentType === "prod");

  const projectsURI = `/t/${selectedTeamSlug}/${projectSlug}`;
  // 0-4 are /t/[team]/[project]/[deploymentName].
  // 5- is the currentView, without query params
  const currentView = router.asPath.split("?")[0].split("/").slice(5).join("/");

  return (
    <>
      <ContextMenu.Item
        icon={<SignalIcon className="h-4 w-4" />}
        label={
          <DeploymentOption
            name={prod?.name || "Select to create a Prod deployment"}
            identifier="Production"
          />
        }
        shortcut={["Ctrl", "Alt", "1"]}
        action={
          prod
            ? `${projectsURI}/${prod.name}/${currentView}`
            : `${projectsURI}/${PROVISION_PROD_PAGE_NAME}`
        }
        blankTarget={false}
      />
      <AllPersonalDeployments
        team={team}
        project={project}
        deployments={deployments}
      />
      {previews.length === 0 && (
        <ContextMenu.Item
          icon={<Pencil2Icon className="h-4 w-4" />}
          label={
            <div className="flex flex-col">
              Preview Deployments
              <NoPreview
                isDemo={project.isDemo}
                arePreviewDeploymentsAvailable={arePreviewDeploymentsAvailable}
              />
            </div>
          }
          proBadge={!arePreviewDeploymentsAvailable}
          blankTarget={false}
          action="https://docs.convex.dev/production/hosting/preview-deployments"
        />
      )}
      {previews.length > 0 && (
        <ContextMenu.Submenu
          label={
            <p className="flex flex-col">
              Previews
              <span className="text-xs text-content-secondary">
                {previews.length} deployment{previews.length === 1 ? "" : "s"}
              </span>
            </p>
          }
          icon={<Pencil2Icon className="h-4 w-4" />}
        >
          {previews
            .sort(
              (a, b) =>
                a.previewIdentifier
                  ?.toLowerCase()
                  .localeCompare(b.previewIdentifier?.toLowerCase() ?? "") ?? 0,
            )
            .map((previewDeployment) => (
              <ContextMenu.Item
                key={previewDeployment.name}
                label={
                  <DeploymentOption
                    identifier={
                      previewDeployment.previewIdentifier ??
                      previewDeployment.name
                    }
                    name={previewDeployment.name}
                  />
                }
                action={`${projectsURI}/${previewDeployment.name}/${currentView}`}
                blankTarget={false}
              />
            ))}
        </ContextMenu.Submenu>
      )}
      {teamMemberDeployments.length > 0 && (
        <ContextMenu.Submenu
          label={
            <p className="flex flex-col">
              Other Deployments
              <span className="text-xs text-content-secondary">
                {teamMemberDeployments.length} deployment
                {teamMemberDeployments.length === 1 ? "" : "s"}
              </span>
            </p>
          }
          icon={<Share1Icon />}
        >
          {teamMemberDeployments
            .sort((a, b) =>
              a.creator.toLowerCase().localeCompare(b.creator.toLowerCase()),
            )
            .map((d) => (
              <ContextMenu.Item
                key={d.name}
                label={
                  <DeploymentOption
                    identifier={`${d.creator}'s dev`}
                    name={d.name}
                  />
                }
                action={`${projectsURI}/${d.name}/${currentView}`}
                blankTarget={false}
              />
            ))}
        </ContextMenu.Submenu>
      )}
      <hr className="my-1 bg-border-transparent" />
      <ContextMenu.Item
        icon={<GearIcon />}
        label={
          <div className="flex flex-col">
            Project Settings
            <span className="text-xs text-content-secondary">
              Manage this project's configuration
            </span>
          </div>
        }
        shortcut={["Ctrl", "Alt", "S"]}
        action={`${projectsURI}/settings`}
        blankTarget={false}
      />
    </>
  );
}

function AllPersonalDeployments({
  team,
  project,
  deployments,
}: {
  project: ProjectDetails;
  team: Team;
  deployments: DeploymentResponse[];
}) {
  const member = useProfile();
  const dev = useDefaultDevDeployment(project.id);
  const router = useRouter();
  const projectSlug = project.slug;

  const selectedTeamSlug = team.slug;

  const projectsURI = `/t/${selectedTeamSlug}/${projectSlug}`;
  // 0-4 are /t/[team]/[project]/[deploymentName].
  // 5- is the currentView
  const currentView = router.asPath.split("?")[0].split("/").slice(5).join("/");
  const allDevDeployments = sortDevDeployments(
    deployments.filter(
      (d: DeploymentResponse) =>
        d.deploymentType === "dev" && d.creator === member?.id,
    ),
  );

  if (allDevDeployments.length === 0) {
    return (
      <ContextMenu.Item
        icon={<CommandLineIcon className="h-4 w-4" />}
        tip={
          <>
            You do not have a personal development deployment for this project
            yet. Run <code className="px-0.5">npx convex dev</code> to provision
            one.
          </>
        }
        tipSide="right"
        label={
          <DeploymentOption
            identifier="Development"
            name="You don't have a dev deployment yet"
          />
        }
        action={`${projectsURI}/${dev?.name}/${currentView}`}
        blankTarget={false}
        disabled
      />
    );
  }
  return (
    <>
      {allDevDeployments
        .filter((d) => (d.kind === "local" ? d.isActive : true))
        .map((d, idx) => (
          <ContextMenu.Item
            key={d.name}
            icon={
              d.kind === "local" ? (
                <CommandLineIcon className="h-4 w-4" />
              ) : (
                <GlobeIcon className="h-4 w-4" />
              )
            }
            shortcut={
              idx + 2 > 9
                ? undefined
                : ["Ctrl", "Alt", (idx + 2).toString() as Key]
            }
            label={
              <DeploymentOption
                identifier={`${d.kind === "local" ? `${d.deviceName}` : "Development (Cloud)"}`}
                name={d.kind === "local" ? `Port ${d.port}` : d.name}
              />
            }
            action={`${projectsURI}/${d.name}/${currentView}`}
            blankTarget={false}
          />
        ))}
    </>
  );
}

function sortDevDeployments(deployments: DeploymentResponse[]) {
  return deployments.sort((a, b) => {
    // Sort inactive local deployments to the end
    if (a.kind === "local" && !a.isActive) {
      return 1;
    }
    if (b.kind === "local" && !b.isActive) {
      return -1;
    }

    // Sort local deployments before cloud deployments
    if (a.kind === "local" && b.kind === "cloud") {
      return -1;
    }
    if (a.kind === "cloud" && b.kind === "local") {
      return 1;
    }

    // Sort by last update time for local deployments
    if (a.kind === "local" && b.kind === "local") {
      return a.lastUpdateTime - b.lastUpdateTime;
    }
    return a.createTime - b.createTime;
  });
}

function DeploymentOption({
  identifier,
  name,
}: {
  identifier: string;
  name: string;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const isOverflowing = useIsOverflowing(ref);

  return (
    <Tooltip
      tip={
        isOverflowing ? (
          <div className="break-all">{identifier}</div>
        ) : undefined
      }
      side="right"
      wrapsButton
    >
      <p className="flex max-w-[20rem] flex-col truncate" ref={ref}>
        {identifier}{" "}
        {name && (
          <span
            className={`text-xs text-content-secondary ${
              !name.includes(" ") ? "font-mono" : ""
            }`}
          >
            {name}
          </span>
        )}
      </p>
    </Tooltip>
  );
}

function NoPreview({
  isDemo,
  arePreviewDeploymentsAvailable,
}: {
  isDemo: boolean;
  arePreviewDeploymentsAvailable: boolean;
}) {
  if (isDemo) {
    return (
      <div className="text-xs text-content-secondary">
        Create a new project to use preview deployments
      </div>
    );
  }
  if (arePreviewDeploymentsAvailable) {
    return (
      <div className="text-xs text-content-secondary">
        Learn how to use preview deployments
      </div>
    );
  }
  return (
    <div className="text-xs text-content-secondary">
      Available on the Pro plan
    </div>
  );
}
