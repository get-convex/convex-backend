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

  const { isCloudDeploymentInSelfHostedDashboard, deploymentName } =
    useIsCloudDeploymentInSelfHostedDashboard();
  const isSelfHostedDeployment =
    isSelfHosted && !isCloudDeploymentInSelfHostedDashboard;

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
        {allowedPages.map((page) => {
          const isCloudOnlyPage = ["backups", "integrations"].includes(page);
          const showInCloudDashboard =
            isCloudOnlyPage && isCloudDeploymentInSelfHostedDashboard;
          const isUnavailableForSelfHosted =
            isCloudOnlyPage && isSelfHostedDeployment;

          return (
            <SidebarLink
              href={
                showInCloudDashboard
                  ? `https://dashboard.convex.dev/d/${deploymentName}/settings/${page}`
                  : `${deploymentsURI}/settings/${page === "url-and-deploy-key" ? "" : page}`
              }
              isActive={page === selectedPage}
              key={page}
              disabled={shouldLock(page) || isUnavailableForSelfHosted}
              tip={
                isUnavailableForSelfHosted
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
              target={showInCloudDashboard ? "_blank" : undefined}
            >
              <div className="flex items-center justify-between gap-2">
                {DEPLOYMENT_SETTINGS_PAGES_AND_NAMES[page]}
                {showInCloudDashboard && <ExternalLinkIcon />}
              </div>
            </SidebarLink>
          );
        })}
        <div className="flex flex-col gap-2 border-t py-2">
          <SidebarLink
            href={
              isCloudDeploymentInSelfHostedDashboard
                ? `https://dashboard.convex.dev/dp/${deploymentName}/settings`
                : `${projectsURI}/settings`
            }
            isActive={false}
            disabled={isSelfHostedDeployment}
            tip={
              isSelfHostedDeployment
                ? "Project settings are not available in self-hosted deployments."
                : undefined
            }
            target={
              isCloudDeploymentInSelfHostedDashboard ? "_blank" : undefined
            }
          >
            <div className="flex items-center justify-between gap-2">
              Project Settings
              <ExternalLinkIcon />
            </div>
          </SidebarLink>
          <SidebarLink
            href={
              isCloudDeploymentInSelfHostedDashboard
                ? `https://dashboard.convex.dev/dp/${deploymentName}/usage`
                : `${teamsURI}/settings/usage`
            }
            query={
              isCloudDeploymentInSelfHostedDashboard
                ? {}
                : {
                    projectSlug: project?.slug ?? "",
                  }
            }
            isActive={false}
            disabled={isSelfHostedDeployment}
            tip={
              isSelfHostedDeployment
                ? "Project usage is not available in self-hosted deployments."
                : undefined
            }
            target={
              isCloudDeploymentInSelfHostedDashboard ? "_blank" : undefined
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

/**
 * Determines if the deployment URL is a default cloud deployment URL.
 *
 * This gives a false negative if the deployment is a cloud deployment with a custom domain.
 */
function useIsCloudDeploymentInSelfHostedDashboard():
  | {
      isCloudDeploymentInSelfHostedDashboard: false;
      deploymentName: undefined;
    }
  | {
      isCloudDeploymentInSelfHostedDashboard: true;
      deploymentName: string;
    } {
  const context = useContext(DeploymentInfoContext);

  if (
    !context.isSelfHosted ||
    !("deploymentUrl" in context) ||
    !context.deploymentUrl
  ) {
    return {
      isCloudDeploymentInSelfHostedDashboard: false,
      deploymentName: undefined,
    };
  }

  const match = context.deploymentUrl.match(
    /^https:\/\/([a-z]+-[a-z]+-[0-9]{3})\.convex\.cloud$/,
  );

  if (!match) {
    return {
      isCloudDeploymentInSelfHostedDashboard: false,
      deploymentName: undefined,
    };
  }

  return {
    isCloudDeploymentInSelfHostedDashboard: true,
    deploymentName: match[1],
  };
}
