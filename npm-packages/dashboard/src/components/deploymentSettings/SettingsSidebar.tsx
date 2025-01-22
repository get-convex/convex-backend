import classNames from "classnames";
import React from "react";
import { SidebarLink, useNents } from "dashboard-common";
import { useCurrentDeployment } from "api/deployments";
import { useCurrentProject } from "api/projects";
import { useCurrentTeam, useTeamEntitlements } from "api/teams";
import { ExternalLinkIcon, LockClosedIcon } from "@radix-ui/react-icons";
import { DeploymentPageTitle } from "elements/DeploymentPageTitle";
import { useTeamUsageState } from "hooks/useTeamUsageState";

export const DEPLOYMENT_SETTINGS_PAGES_AND_NAMES = {
  "url-and-deploy-key": "URL & Deploy Key",
  "environment-variables": "Environment Variables",
  authentication: "Authentication",
  snapshots: "Snapshot Import & Export",
  backups: "Backup & Restore",
  components: "Components",
  integrations: "Integrations",
  "pause-deployment": "Pause Deployment",
};

export type SettingsPageKind = keyof typeof DEPLOYMENT_SETTINGS_PAGES_AND_NAMES;

const DEPLOYMENT_SETTINGS_PAGES = Object.keys(
  DEPLOYMENT_SETTINGS_PAGES_AND_NAMES,
) as SettingsPageKind[];

export function SettingsSidebar({
  selectedPage,
}: {
  selectedPage: SettingsPageKind;
}) {
  const allowedPages = useAllowedPages();

  const team = useCurrentTeam();
  const project = useCurrentProject();
  const deployment = useCurrentDeployment();

  const entitlements = useTeamEntitlements(team?.id);
  // Hide the badge until entitlements are loaded
  const logStreamingEntitlementGranted =
    entitlements?.logStreamingEnabled ?? true;
  const streamingExportEntitlementGranted =
    entitlements?.streamingExportEnabled ?? true;
  const teamUsageState = useTeamUsageState(team?.id ?? null);

  const shouldLock = (page: string) =>
    page === "pause-deployment" &&
    (teamUsageState === "Paused" || teamUsageState === "Disabled");

  return (
    <>
      <DeploymentPageTitle
        title={DEPLOYMENT_SETTINGS_PAGES_AND_NAMES[selectedPage]}
      />
      <div
        className={classNames(
          "flex gap-2 h-full min-w-60 flex-col bg-background-secondary md:border-r md:p-4 overflow-y-auto scrollbar",
        )}
      >
        {/* On larger screens, this is a sidebar and not a popover menu. */}
        {allowedPages.map((page) => (
          <SidebarLink
            href={`/t/${team?.slug}/${project?.slug}/${
              deployment?.name
            }/settings/${page === "url-and-deploy-key" ? "" : page}`}
            isActive={page === selectedPage}
            key={page}
            disabled={shouldLock(page)}
            Icon={shouldLock(page) ? LockClosedIcon : undefined}
            proBadge={
              page === "integrations" &&
              !(
                logStreamingEntitlementGranted &&
                streamingExportEntitlementGranted
              )
            }
          >
            {DEPLOYMENT_SETTINGS_PAGES_AND_NAMES[page]}
          </SidebarLink>
        ))}
        <div className="flex flex-col gap-2 border-t py-2">
          <SidebarLink
            href={`/t/${team?.slug}/${project?.slug}/settings`}
            isActive={false}
          >
            <div className="flex items-center justify-between">
              Project Settings
              <ExternalLinkIcon />
            </div>
          </SidebarLink>
          <SidebarLink
            href={`/t/${team?.slug}/settings/usage`}
            query={{ projectSlug: project?.slug || "" }}
            isActive={false}
          >
            <div className="flex items-center justify-between">
              Project Usage
              <ExternalLinkIcon />
            </div>
          </SidebarLink>
        </div>
      </div>
    </>
  );
}

function useAllowedPages() {
  const { nents } = useNents();

  let pages = DEPLOYMENT_SETTINGS_PAGES;

  if (!nents || nents.length === 0) {
    pages = pages.filter((d) => d !== "components");
  }

  pages = pages.filter((d) => d !== "snapshots");

  return pages;
}
