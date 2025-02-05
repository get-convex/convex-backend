import classNames from "classnames";
import React, { useContext } from "react";
import { ExternalLinkIcon, LockClosedIcon } from "@radix-ui/react-icons";
import { DeploymentPageTitle } from "@common/elements/DeploymentPageTitle";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { SidebarLink } from "@common/elements/Sidebar";
import { useNents } from "@common/lib/useNents";

export const DEPLOYMENT_SETTINGS_PAGES_AND_NAMES = {
  "url-and-deploy-key": "URL & Deploy Key",
  "environment-variables": "Environment Variables",
  authentication: "Authentication",
  snapshots: "Snapshot Import & Export",
  components: "Components",
  backups: "Backup & Restore",
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

  const {
    isSelfHosted,
    useCurrentTeam,
    useCurrentProject,
    useTeamUsageState,
    useTeamEntitlements,
    teamsURI,
    projectsURI,
    deploymentsURI,
  } = useContext(DeploymentInfoContext);

  const team = useCurrentTeam();
  const project = useCurrentProject();

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
            href={`${deploymentsURI}settings/${page === "url-and-deploy-key" ? "" : page}`}
            isActive={page === selectedPage}
            key={page}
            disabled={
              shouldLock(page) ||
              (isSelfHosted &&
                ["backups", "integrations", "pause-deployment"].includes(page))
            }
            tip={
              ["backups", "integrations", "pause-deployment"].includes(page) &&
              isSelfHosted
                ? `The ${DEPLOYMENT_SETTINGS_PAGES_AND_NAMES[page]} feature is not currently available in self-hosted deployments.`
                : undefined
            }
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
            href={`${projectsURI}/settings`}
            isActive={false}
            disabled={isSelfHosted}
            tip={
              isSelfHosted
                ? "Project settings are not available in self-hosted deployments."
                : undefined
            }
          >
            <div className="flex items-center justify-between gap-2">
              Project Settings
              <ExternalLinkIcon />
            </div>
          </SidebarLink>
          <SidebarLink
            href={`${teamsURI}/settings/usage`}
            query={{ projectSlug: project?.slug || "" }}
            isActive={false}
            disabled={isSelfHosted}
            tip={
              isSelfHosted
                ? "Project usage is not available in self-hosted deployments."
                : undefined
            }
          >
            <div className="flex items-center justify-between gap-2">
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
