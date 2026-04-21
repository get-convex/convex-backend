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
import { DeploymentResponse, ProjectDetails, TeamResponse } from "generatedApi";
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
  deployments: (PlatformDeploymentResponse | DeploymentResponse)[];
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
  const teamMemberDevDeployments = deployments.filter(
    (d): d is PlatformDeploymentResponse & { kind: "cloud" } =>
      d.kind === "cloud" &&
      d.deploymentType === "dev" &&
      d.creator !== member?.id,
  );
  const nonDefaultTeamDevs = teamMemberDevDeployments.filter(
    (d) => !d.isDefault,
  );

  function mapTeamDev(d: PlatformDeploymentResponse & { kind: "cloud" }) {
    const whose = members?.find((tm) => tm.id === d.creator);
    return {
      name: d.name,
      creator: whose?.name || whose?.email || "Teammate",
      isDefault: d.isDefault,
      reference: d.reference,
    };
  }

  // If there are too many non-default team devs, move them all to "Other Deployments"
  const showNonDefaultTeamDevsInMainMenu = nonDefaultTeamDevs.length < 10;
  const otherDeployments = teamMemberDevDeployments
    .filter((d) => (showNonDefaultTeamDevsInMainMenu ? d.isDefault : true))
    .map(mapTeamDev)
    .sort((a, b) => {
      if (a.isDefault !== b.isDefault) {
        return a.isDefault ? 1 : -1;
      }
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
      <MainMenuDevItems
        team={team}
        project={project}
        deployments={deployments}
        nonDefaultTeamDevs={
          showNonDefaultTeamDevsInMainMenu ? nonDefaultTeamDevs : []
        }
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
          action="https://docs.convex.dev/production/hosting/multiple-deployments#preview"
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
        disabled={otherDeployments.length === 0}
        label={
          <p className="flex flex-col">
            Other Deployments
            {otherDeployments.length === 0 ? (
              <span className="text-xs text-content-secondary">
                <span className="text-content-tertiary">
                  Team member deployments appear here
                </span>
              </span>
            ) : (
              <span className="text-xs text-content-secondary">
                {`${otherDeployments.length} deployment${otherDeployments.length === 1 ? "" : "s"}`}
              </span>
            )}
          </p>
        }
        icon={<Share1Icon />}
      >
        {otherDeployments.map((d) => (
          <ContextMenu.Item
            key={d.name}
            label={
              <DeploymentOption
                identifier={d.isDefault ? `${d.creator}'s dev` : d.reference}
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

function MainMenuDevItems({
  team,
  project,
  deployments,
  nonDefaultTeamDevs,
}: {
  project: ProjectDetails;
  team: TeamResponse;
  deployments: (PlatformDeploymentResponse | DeploymentResponse)[];
  nonDefaultTeamDevs: (PlatformDeploymentResponse | DeploymentResponse)[];
}) {
  const member = useProfile();
  const router = useRouter();
  const projectSlug = project.slug;

  const selectedTeamSlug = team.slug;

  const projectsURI = `/t/${selectedTeamSlug}/${projectSlug}`;
  // 0-4 are /t/[team]/[project]/[deploymentName].
  // 5- is the currentView
  const currentView = router.asPath.split("?")[0].split("/").slice(5).join("/");
  const personalDevs = sortDevDeployments(
    deployments.filter(
      (d: PlatformDeploymentResponse | DeploymentResponse) =>
        d.deploymentType === "dev" && d.creator === member?.id,
    ),
  ).filter((d) => (d.kind === "local" ? d.isActive : true));

  const allDevs = [...personalDevs, ...nonDefaultTeamDevs];
  // When personalDevs is empty, the "create dev" item occupies slot 2,
  // so allDevs items must start at slot 3 to avoid a duplicate shortcut.
  const devIdxOffset = personalDevs.length === 0 ? 1 : 0;

  return (
    <>
      {personalDevs.length === 0 && (
        <ContextMenu.Item
          icon={<CommandLineIcon className="h-4 w-4" />}
          label={
            <DeploymentOption
              name="Select to create your dev deployment."
              identifier="Development"
            />
          }
          shortcut={["Ctrl", "Alt", "2"]}
          action={`${projectsURI}/${PROVISION_DEV_PAGE_NAME}`}
          blankTarget={false}
        />
      )}
      {allDevs.map((d, idx) => (
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
            idx + devIdxOffset + 2 > 9
              ? undefined
              : ["Ctrl", "Alt", (idx + devIdxOffset + 2).toString() as Key]
          }
          label={
            <DeploymentOption
              identifier={
                d.kind === "local"
                  ? d.deviceName
                  : d.isDefault
                    ? "Development (Cloud)"
                    : d.reference
              }
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

function sortDevDeployments(
  deployments: (PlatformDeploymentResponse | DeploymentResponse)[],
) {
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
