import { useTeamMembers } from "api/teams";
import { useProfile } from "api/profile";
import { useRouter } from "next/router";
import React, { useRef } from "react";
import {
  CommandLineIcon,
  SignalIcon,
  WrenchIcon,
} from "@heroicons/react/24/outline";
import { Tooltip } from "@ui/Tooltip";
import {
  GearIcon,
  GlobeIcon,
  Pencil2Icon,
  Share1Icon,
} from "@radix-ui/react-icons";
import { PlatformDeploymentResponse } from "@convex-dev/platform/managementApi";
import { ProjectDetails, TeamResponse } from "generatedApi";
import {
  PROVISION_DEV_PAGE_NAME,
  PROVISION_PROD_PAGE_NAME,
} from "@common/lib/deploymentContext";
import { useIsOverflowing } from "@common/lib/useIsOverflowing";
import { ContextMenu } from "@common/features/data/components/ContextMenu";
import { Key } from "@ui/KeyboardShortcut";

export function DeploymentMenuOptions({
  team,
  project,
  deployments,
}: {
  team: TeamResponse;
  project: ProjectDetails;
  deployments: PlatformDeploymentResponse[];
}) {
  const member = useProfile();
  const router = useRouter();

  const prods = deployments
    .filter(
      (d): d is PlatformDeploymentResponse & { kind: "cloud" } =>
        d.deploymentType === "prod",
    )
    .sort((a, b) => {
      // Default deployment comes first
      if (a.isDefault && !b.isDefault) return -1;
      if (!a.isDefault && b.isDefault) return 1;
      // Then sort by createTime (newest first)
      return b.createTime - a.createTime;
    });
  const previews = deployments
    .filter((d) => d.deploymentType === "preview")
    .sort((a, b) => b.createTime - a.createTime);
  const custom = deployments
    .filter((d) => d.deploymentType === "custom")
    .sort((a, b) => b.createTime - a.createTime);

  const members = useTeamMembers(team.id);
  const teamMemberDeployments = deployments
    .filter(
      (d): d is PlatformDeploymentResponse & { kind: "cloud" } =>
        d.kind === "cloud" &&
        d.deploymentType === "dev" &&
        d.creator !== member?.id,
    )
    .map((d) => {
      const whose = members?.find((tm) => tm.id === d.creator);
      return {
        name: d.name,
        creator: whose?.name || whose?.email || "Teammate",
        isDefault: d.isDefault,
      };
    })
    .sort((a, b) => {
      // Non-default deployments come first
      if (a.isDefault !== b.isDefault) {
        return a.isDefault ? 1 : -1;
      }
      // Then sort by creator name
      return a.creator.toLowerCase().localeCompare(b.creator.toLowerCase());
    });

  const projectSlug = project.slug;

  const selectedTeamSlug = team.slug;

  // Show as single item only if there's exactly one prod deployment and it's the default
  const showProdAsSingleItem =
    prods.length === 1 && prods[0].kind === "cloud" && prods[0].isDefault;
  const singleDefaultProd = showProdAsSingleItem ? prods[0] : undefined;

  const projectsURI = `/t/${selectedTeamSlug}/${projectSlug}`;
  // 0-4 are /t/[team]/[project]/[deploymentName].
  // 5- is the currentView, without query params
  const currentView = router.asPath.split("?")[0].split("/").slice(5).join("/");

  return (
    <>
      {/* No prod deployments: show option to create one */}
      {prods.length === 0 && (
        <ContextMenu.Item
          icon={<SignalIcon className="h-4 w-4" />}
          label={
            <DeploymentOption
              name="Select to create a Prod deployment"
              identifier="Production"
            />
          }
          shortcut={["Ctrl", "Alt", "1"]}
          action={`${projectsURI}/${PROVISION_PROD_PAGE_NAME}`}
          blankTarget={false}
        />
      )}
      {/* Single default prod: show as single item */}
      {singleDefaultProd && (
        <ContextMenu.Item
          icon={<SignalIcon className="h-4 w-4" />}
          label={
            <DeploymentOption
              name={singleDefaultProd.name}
              identifier="Production"
            />
          }
          shortcut={["Ctrl", "Alt", "1"]}
          action={`${projectsURI}/${singleDefaultProd.name}/${currentView}`}
          blankTarget={false}
        />
      )}
      {/* Multiple prods or single non-default: show as submenu */}
      {prods.length > 0 && !showProdAsSingleItem && (
        <ContextMenu.Submenu
          label={
            <p className="flex flex-col">
              Production
              <span className="text-xs text-content-secondary">
                {prods.length} deployment{prods.length === 1 ? "" : "s"}
              </span>
            </p>
          }
          icon={<SignalIcon className="h-4 w-4" />}
        >
          {prods.map((prodDeployment) => (
            <ContextMenu.Item
              key={prodDeployment.name}
              label={
                <DeploymentOption
                  identifier={prodDeployment.reference}
                  name={prodDeployment.name}
                />
              }
              shortcut={
                prodDeployment.isDefault ? ["Ctrl", "Alt", "1"] : undefined
              }
              action={`${projectsURI}/${prodDeployment.name}/${currentView}`}
              blankTarget={false}
            />
          ))}
        </ContextMenu.Submenu>
      )}
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
              <div className="text-xs text-content-secondary">
                Learn how to use preview deployments
              </div>
            </div>
          }
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
      {custom.length > 0 && (
        <ContextMenu.Submenu
          label={
            <p className="flex flex-col">
              Custom Deployments
              <span className="text-xs text-content-secondary">
                {custom.length} deployment{custom.length === 1 ? "" : "s"}
              </span>
            </p>
          }
          icon={<WrenchIcon className="h-4 w-4" />}
        >
          {custom.map((customDeployment) => (
            <ContextMenu.Item
              key={customDeployment.name}
              label={
                <DeploymentOption
                  identifier={
                    customDeployment.kind === "cloud"
                      ? customDeployment.reference
                      : "" // should never happen
                  }
                  name={customDeployment.name}
                />
              }
              action={`${projectsURI}/${customDeployment.name}/${currentView}`}
              blankTarget={false}
            />
          ))}
        </ContextMenu.Submenu>
      )}
      <ContextMenu.Submenu
        disabled={teamMemberDeployments.length === 0}
        label={
          <p className="flex flex-col">
            Other Deployments
            {teamMemberDeployments.length === 0 ? (
              <span className="text-xs text-content-secondary">
                <span className="text-content-tertiary">
                  Team member deployments appear here
                </span>
              </span>
            ) : (
              <span className="text-xs text-content-secondary">
                {`${teamMemberDeployments.length} deployment${teamMemberDeployments.length === 1 ? "" : "s"}`}
              </span>
            )}
          </p>
        }
        icon={<Share1Icon />}
      >
        {teamMemberDeployments.map((d) => (
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
  team: TeamResponse;
  deployments: PlatformDeploymentResponse[];
}) {
  const member = useProfile();
  const router = useRouter();
  const projectSlug = project.slug;

  const selectedTeamSlug = team.slug;

  const projectsURI = `/t/${selectedTeamSlug}/${projectSlug}`;
  // 0-4 are /t/[team]/[project]/[deploymentName].
  // 5- is the currentView
  const currentView = router.asPath.split("?")[0].split("/").slice(5).join("/");
  const allDevDeployments = sortDevDeployments(
    deployments.filter(
      (d: PlatformDeploymentResponse) =>
        d.deploymentType === "dev" && d.creator === member?.id,
    ),
  );

  if (allDevDeployments.length === 0) {
    return (
      <ContextMenu.Item
        icon={<CommandLineIcon className="h-4 w-4" />}
        label={
          <DeploymentOption
            name="Select to create a Dev deployment."
            identifier="Development"
          />
        }
        shortcut={["Ctrl", "Alt", "2"]}
        action={`${projectsURI}/${PROVISION_DEV_PAGE_NAME}`}
        blankTarget={false}
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

function sortDevDeployments(deployments: PlatformDeploymentResponse[]) {
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

    // Sort by create time
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
      asChild
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
