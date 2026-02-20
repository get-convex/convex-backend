import {
  CommandLineIcon,
  SignalIcon,
  WrenchIcon,
} from "@heroicons/react/24/outline";
import { CaretSortIcon, GearIcon, Pencil2Icon } from "@radix-ui/react-icons";
import { useCurrentDeployment, useDeployments } from "api/deployments";
import { useCurrentTeam, useTeamEntitlements, useTeamMembers } from "api/teams";
import { useProfile } from "api/profile";
import { useRememberLastViewedDeploymentForProject } from "hooks/useLastViewed";
import { cn } from "@ui/cn";
import { useRouter } from "next/router";
import { PlatformDeploymentResponse } from "@convex-dev/platform/managementApi";
import { ProjectDetails, DeploymentType } from "generatedApi";
import { Button } from "@ui/Button";
import { ContextMenu } from "@common/features/data/components/ContextMenu";
import { DeploymentMenuOptions } from "components/header/ProjectSelector/DeploymentMenuOptions";
import { useCurrentProject } from "api/projects";
import { useRef, useState, useEffect } from "react";
import {
  PROVISION_DEV_PAGE_NAME,
  PROVISION_PROD_PAGE_NAME,
} from "@common/lib/deploymentContext";
import { useHotkeys } from "react-hotkeys-hook";
import { useListVanityDomains } from "api/vanityDomains";
import { useQuery } from "convex/react";
import udfs from "@common/udfs";
import { DeploymentProvider } from "components/projectSettings/CustomDomains";
import { useContainerWidth } from "../hooks/useContainerWidth";

function DeploymentDomainInfo({
  deployment,
  deployments,
  whoseName,
}: {
  deployment: PlatformDeploymentResponse;
  deployments: PlatformDeploymentResponse[];
  whoseName: string | null;
}) {
  const team = useCurrentTeam();
  const hasEntitlement = !!useTeamEntitlements(team?.id)?.customDomainsEnabled;
  const domains = useListVanityDomains(
    hasEntitlement ? deployment?.name : undefined,
  );
  const vanityCloudDomains = domains?.filter(
    (d) => d.requestDestination === "convexCloud",
  );
  const canonicalCloudUrl = useQuery(udfs.convexCloudUrl.default);
  const vanityDomain =
    vanityCloudDomains?.find((d) => d.domain === canonicalCloudUrl) ||
    vanityCloudDomains?.[0];
  return (
    <DeploymentLabel
      deployment={deployment}
      whoseName={whoseName}
      deployments={deployments}
      vanityUrl={
        vanityDomain?.verificationTime ? vanityDomain.domain : undefined
      }
    />
  );
}

function DeploymentLabelWrapper({
  deployment,
  whoseName,
  deployments,
  deploymentName,
}: {
  deployment: PlatformDeploymentResponse;
  whoseName: string | null;
  deployments: PlatformDeploymentResponse[];
  deploymentName: string;
}) {
  return (
    <DeploymentProvider deploymentName={deploymentName}>
      <DeploymentDomainInfo
        deployment={deployment}
        whoseName={whoseName}
        deployments={deployments}
      />
    </DeploymentProvider>
  );
}

// TODO(ENG-10340) Use references here to disambiguate non-default dev/prod deployments
export function DeploymentDisplay({ project }: { project: ProjectDetails }) {
  const router = useRouter();

  const deployment = useCurrentDeployment();
  const member = useProfile();

  useRememberLastViewedDeploymentForProject(project.slug, deployment?.name);

  const teamMembers = useTeamMembers(project.teamId);

  const isProdSelected = deployment?.deploymentType === "prod";
  const isPreview = deployment?.deploymentType === "preview";
  const isDoneLoading =
    isProdSelected || // prod deploys are straightforward
    isPreview || // preview deployments don't require additional info
    (deployment !== undefined &&
      member !== undefined &&
      teamMembers !== undefined); // info required to render dev deploy

  const whose = teamMembers?.find((tm) => tm.id === deployment?.creator);
  const profile = useProfile();
  const whoseName =
    whose?.email === profile?.email
      ? null
      : whose?.name || whose?.email || "Teammate";

  const isProjectSettings = router.route.endsWith("/[project]/settings");
  const team = useCurrentTeam();
  const currentProject = useCurrentProject();

  const { deployments: deploymentData } = useDeployments(currentProject?.id);
  const deployments = deploymentData || [];
  const projectSlug = currentProject?.slug;
  const selectedTeamSlug = team?.slug;
  const projectsURI = `/t/${selectedTeamSlug}/${projectSlug}`;
  const currentView = router.asPath.split("/").slice(5).join("/");
  const devDeployments = deployments.filter(
    (d) => d.deploymentType === "dev" && d.creator === member?.id,
  );
  const defaultProd = deployments.find(
    (d) => d.kind === "cloud" && d.deploymentType === "prod" && d.isDefault,
  );

  // Hotkeys
  useHotkeys(
    "ctrl+alt+1",
    () => {
      if (defaultProd) {
        void router.push(`${projectsURI}/${defaultProd.name}/${currentView}`);
      } else {
        void router.push(`${projectsURI}/${PROVISION_PROD_PAGE_NAME}`);
      }
    },
    [defaultProd, projectsURI, currentView],
  );
  useHotkeys(
    devDeployments.length === 0 ? [`ctrl+alt+2`] : [],
    () => {
      void router.push(`${projectsURI}/${PROVISION_DEV_PAGE_NAME}`);
    },
    [devDeployments, projectsURI],
  );
  useHotkeys(
    Array.from({ length: devDeployments.length }, (_, idx) => [
      `ctrl+alt+${idx + 2}`,
    ]).flat(),
    (event, handler) => {
      const keyStr = handler.keys?.[0] || "";
      if (keyStr) {
        const devIdx = parseInt(keyStr.split("+").pop() || "", 10) - 2;
        if (devIdx >= 0 && devIdx < devDeployments.length) {
          void router.push(
            `${projectsURI}/${devDeployments[devIdx].name}/${currentView}`,
          );
        }
      }
    },
    [devDeployments, projectsURI, currentView],
  );
  useHotkeys(
    "ctrl+alt+s",
    () => {
      void router.push(`${projectsURI}/settings`);
    },
    [projectsURI],
  );

  // ContextMenu trigger state
  const buttonRef = useRef<HTMLButtonElement>(null);
  const [menuTarget, setMenuTarget] = useState<{ x: number; y: number } | null>(
    null,
  );
  const openMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    if (buttonRef.current) {
      const rect = buttonRef.current.getBoundingClientRect();
      setMenuTarget({ x: rect.left, y: rect.bottom });
    }
  };
  const closeMenu = () => setMenuTarget(null);

  return isProjectSettings ? (
    team && currentProject ? (
      <div
        key="projectSettings"
        className="my-2 mr-px flex grow items-stretch overflow-visible rounded-full bg-background-secondary"
      >
        <Button
          variant="unstyled"
          className={cn(
            "flex h-full items-center gap-2 rounded-full px-3",
            "border bg-background-secondary text-content-primary",
            "truncate text-sm font-medium transition-opacity hover:bg-background-tertiary",
            menuTarget && "border-border-selected bg-background-tertiary",
          )}
          ref={buttonRef}
          tabIndex={0}
          role="button"
          aria-haspopup="menu"
          aria-expanded={!!menuTarget}
          onClick={openMenu}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") openMenu(e as any);
          }}
        >
          <GearIcon className="size-4 min-w-4" />
          <span className="max-w-24 truncate sm:contents">
            Project settings
          </span>
          <CaretSortIcon className="ml-auto size-5 bg-transparent" />
          <ContextMenu target={menuTarget} onClose={closeMenu}>
            <DeploymentMenuOptions
              team={team}
              project={currentProject}
              deployments={deployments}
            />
          </ContextMenu>
        </Button>
      </div>
    ) : null
  ) : !isDoneLoading ? null : (
    <DeploymentLabelWrapper
      deployment={deployment}
      whoseName={whoseName}
      deployments={deployments}
      deploymentName={deployment.name}
    />
  );
}

export function DeploymentLabel({
  whoseName,
  deployment,
  deployments,
  vanityUrl,
}: {
  deployment: PlatformDeploymentResponse;
  deployments: PlatformDeploymentResponse[];
  whoseName: string | null;
  vanityUrl?: string;
}) {
  const team = useCurrentTeam();
  const project = useCurrentProject();
  const buttonRef = useRef<HTMLButtonElement>(null);
  const [menuTarget, setMenuTarget] = useState<{ x: number; y: number } | null>(
    null,
  );
  const openMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    if (buttonRef.current) {
      const rect = buttonRef.current.getBoundingClientRect();
      setMenuTarget({ x: rect.left, y: rect.bottom });
    }
  };
  const closeMenu = () => setMenuTarget(null);

  const [containerRef, containerWidth] = useContainerWidth<HTMLDivElement>();

  // Hysteresis buffer for domain display
  const DOMAIN_SHOW_THRESHOLD = 350;
  const DOMAIN_HIDE_THRESHOLD = 320;
  const [showDomain, setShowDomain] = useState(
    containerWidth > DOMAIN_SHOW_THRESHOLD,
  );
  const prevShowDomain = useRef(showDomain);

  useEffect(() => {
    if (showDomain && containerWidth < DOMAIN_HIDE_THRESHOLD) {
      setShowDomain(false);
    }
    if (!showDomain && containerWidth > DOMAIN_SHOW_THRESHOLD) {
      setShowDomain(true);
    }
    prevShowDomain.current = showDomain;
  }, [containerWidth, showDomain]);

  // Estimate minimum widths (adjust as needed)
  const minTypeWidth = 100;
  const minDomainWidth = 180;
  const minNameWidth = 120;
  const padding = 48; // for icons, gaps, caret, etc.

  // Decide what to show
  const showType = true;
  const showName =
    containerWidth > minTypeWidth + minDomainWidth + minNameWidth + padding;

  if (!team || !project) {
    return null;
  }
  return (
    <div
      ref={containerRef}
      className={cn(
        "my-2 flex min-w-24 grow overflow-visible p-px",
        "overflow-auto",
      )}
    >
      <Button
        variant="unstyled"
        id="select-deployment"
        className={cn(
          "flex h-[2.3125rem] items-center gap-2 truncate rounded-full border text-sm font-medium transition-opacity hover:opacity-80",
          menuTarget && "opacity-80",
          "focus-visible:ring-1 focus-visible:ring-border-selected focus-visible:outline-hidden",
          getBackgroundColor(deployment.deploymentType),
        )}
        type="button"
        ref={buttonRef}
        aria-haspopup="menu"
        aria-expanded={!!menuTarget}
        tabIndex={0}
        onClick={openMenu}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") openMenu(e as any);
        }}
      >
        <div className="flex h-full w-full animate-fadeInFromLoading cursor-pointer items-center gap-1 px-4">
          {showType && (
            <>
              {deployment.deploymentType === "dev" ? (
                <CommandLineIcon className="size-4 min-w-4" />
              ) : deployment.deploymentType === "prod" ? (
                <SignalIcon className="size-4 min-w-4" />
              ) : deployment.deploymentType === "preview" ? (
                <Pencil2Icon className="size-4 min-w-4" />
              ) : deployment.deploymentType === "custom" ? (
                <WrenchIcon className="size-4 min-w-4" />
              ) : null}
              <span className="max-w-24 truncate sm:contents">
                {getDeploymentLabel({
                  deployment,
                  whoseName,
                })}
              </span>
            </>
          )}
          {showDomain && deployment.kind === "cloud" && vanityUrl && (
            <>
              <span
                className="animate-fadeInFromLoading px-0.5 font-normal"
                role="separator"
              >
                •
              </span>
              <span
                className="block max-w-60 animate-fadeInFromLoading truncate font-mono font-normal"
                title={vanityUrl}
              >
                {vanityUrl}
              </span>
            </>
          )}
          {showName && deployment.kind === "cloud" && (
            <>
              <span
                className="animate-fadeInFromLoading px-0.5 font-normal"
                role="separator"
              >
                •
              </span>
              <span className="animate-fadeInFromLoading font-mono font-normal">
                {deployment.name}
              </span>
            </>
          )}
          {showName && deployment.kind === "local" && (
            <>
              <span
                className="animate-fadeInFromLoading px-0.5 font-normal"
                role="separator"
              >
                •
              </span>
              <span className="animate-fadeInFromLoading font-mono text-sm font-normal">
                Port {deployment.port}
              </span>
            </>
          )}
          <CaretSortIcon
            className={cn(
              "ml-auto size-5 shrink-0",
              getBackgroundColor(deployment.deploymentType),
              "bg-transparent",
            )}
          />
          <ContextMenu target={menuTarget} onClose={closeMenu}>
            <DeploymentMenuOptions
              team={team}
              project={project}
              deployments={deployments}
            />
          </ContextMenu>
        </div>
      </Button>
    </div>
  );
}

export function getBackgroundColor(deploymentType: DeploymentType): string {
  switch (deploymentType) {
    case "prod":
      return "border-purple-600 dark:border-purple-100 bg-purple-100 text-purple-600 dark:bg-purple-700 dark:text-purple-100";
    case "preview":
      return "border-orange-600 dark:border-orange-400 bg-orange-100 text-orange-600 dark:bg-orange-900 dark:text-orange-400";
    case "dev":
      return "border-green-600 dark:border-green-400 bg-green-100 text-green-600 dark:bg-green-900 dark:text-green-400";
    case "custom":
      return "border-neutral-4 dark:border-neutral-6 bg-neutral-1 text-neutral-11 dark:bg-neutral-12 dark:text-neutral-2";
    default: {
      deploymentType satisfies never;
      return "";
    }
  }
}

export function getDeploymentLabel({
  deployment,
  whoseName,
}: {
  deployment: PlatformDeploymentResponse;
  whoseName: string | null; // null = mine
}): string {
  switch (deployment.deploymentType) {
    case "prod":
      return "Production";
    case "preview":
      return `Preview: ${deployment.previewIdentifier || "Unknown"}`;
    case "dev": {
      if (deployment.kind === "local") {
        return deployment.deviceName;
      }
      return whoseName === null ? "Development (Cloud)" : `${whoseName}’s Dev`;
    }
    case "custom":
      return "Custom";
    default: {
      deployment.deploymentType satisfies never;
      return "";
    }
  }
}
