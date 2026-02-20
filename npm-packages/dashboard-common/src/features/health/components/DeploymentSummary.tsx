import { Sheet } from "@ui/Sheet";
import { cn } from "@ui/cn";
import { useQuery } from "convex/react";
import udfs from "@common/udfs";
import { TimestampDistance } from "@common/elements/TimestampDistance";
import { PlatformDeploymentResponse } from "@convex-dev/platform/managementApi";
import {
  CommandLineIcon,
  SignalIcon,
  WrenchIcon,
} from "@heroicons/react/24/outline";
import {
  GlobeIcon,
  Pencil2Icon,
  RocketIcon,
  ArchiveIcon,
  CubeIcon,
  CodeIcon,
  ExternalLinkIcon,
} from "@radix-ui/react-icons";
import Link from "next/link";
import { useContext, useEffect, useState } from "react";
import semver from "semver";
import { Button } from "@ui/Button";
import { Tooltip } from "@ui/Tooltip";
import { Spinner } from "@ui/Spinner";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";

function getBackgroundColor(
  deploymentType: PlatformDeploymentResponse["deploymentType"],
): string {
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

function useLatestConvexVersion(currentVersion: string | undefined) {
  const [hasUpdate, setHasUpdate] = useState(false);
  const [latestVersion, setLatestVersion] = useState<string>();

  useEffect(() => {
    let isMounted = true;

    async function checkVersion() {
      try {
        const response = await fetch(
          "https://registry.npmjs.org/convex/latest",
        );
        if (!response.ok) return;

        const data = await response.json();
        if (!isMounted) return;

        setLatestVersion(data.version);

        if (currentVersion && data.version) {
          const currentSemver = semver.parse(currentVersion);
          const latestSemver = semver.parse(data.version);
          if (!currentSemver || !latestSemver) return;
          const isHigherMinor =
            latestSemver.major === currentSemver.major &&
            latestSemver.minor > currentSemver.minor;
          const isHigherMajor = latestSemver.major > currentSemver.major;
          const hasNewVersion = isHigherMajor || isHigherMinor;
          setHasUpdate(hasNewVersion);
        }
      } catch {
        // Swallow any errors and don't show update notice
      }
    }

    if (currentVersion) {
      void checkVersion();
    }

    return () => {
      isMounted = false;
    };
  }, [currentVersion]);

  return { hasUpdate, latestVersion };
}

function DeploymentIcon({
  deployment,
  className,
}: {
  deployment: PlatformDeploymentResponse;
  className?: string;
}) {
  if (deployment.deploymentType === "dev") {
    return (
      <CommandLineIcon
        className={className}
        aria-label="Development deployment"
      />
    );
  }
  if (deployment.deploymentType === "prod") {
    return (
      <SignalIcon className={className} aria-label="Production deployment" />
    );
  }
  if (deployment.deploymentType === "preview") {
    return (
      <Pencil2Icon className={className} aria-label="Preview deployment" />
    );
  }
  if (deployment.deploymentType === "custom") {
    return <WrenchIcon className={className} aria-label="Custom deployment" />;
  }
  return null;
}

function getDeploymentTypeLabel(
  deploymentType: PlatformDeploymentResponse["deploymentType"],
): string {
  switch (deploymentType) {
    case "prod":
      return "Production";
    case "preview":
      return "Preview";
    case "dev":
      return "Development";
    case "custom":
      return "Custom";
    default: {
      deploymentType satisfies never;
      return "";
    }
  }
}

export function DeploymentSummary({
  deployment,
  teamSlug,
  projectSlug,
  lastBackupTime,
  creatorId,
  creatorName,
  regions,
}: {
  deployment: PlatformDeploymentResponse;
  teamSlug: string;
  projectSlug: string;
  lastBackupTime?: number | null;
  creatorId?: number;
  creatorName?: string;
  regions?: Array<{ name: string; displayName: string }>;
}) {
  const { TeamMemberLink } = useContext(DeploymentInfoContext);
  const lastPushEvent = useQuery(udfs.deploymentEvents.lastPushEvent, {});
  const convexCloudUrl = useQuery(udfs.convexCloudUrl.default, {});
  const convexSiteUrl = useQuery(udfs.convexSiteUrl.default, {});
  const serverVersion = useQuery(udfs.getVersion.default);
  const { hasUpdate, latestVersion } = useLatestConvexVersion(
    serverVersion || undefined,
  );

  const backupSettingsUrl = `/t/${teamSlug}/${projectSlug}/${deployment.name}/settings/backups`;

  // Get display name for region
  const regionDisplayName =
    deployment.kind === "cloud"
      ? regions?.find((r) => r.name === deployment.region)?.displayName ||
        deployment.region
      : undefined;

  // Check if we're still loading critical data
  const isLoading =
    lastPushEvent === undefined ||
    serverVersion === undefined ||
    (deployment.kind === "cloud" &&
      (convexCloudUrl === undefined ||
        convexSiteUrl === undefined ||
        lastBackupTime === undefined));

  if (isLoading) {
    return (
      <Sheet className="flex w-fit flex-col bg-transparent" padding={false}>
        <div className="flex min-h-[7.5rem] min-w-[32rem] items-center justify-center rounded-lg bg-background-secondary p-2 py-3">
          <div>
            <Spinner className="size-8" />
          </div>
        </div>
      </Sheet>
    );
  }

  return (
    <Sheet className="flex w-fit flex-col bg-transparent" padding={false}>
      <div className="flex flex-col lg:flex-row">
        {/* Main deployment info */}
        <div className="flex flex-col gap-4 rounded-l-lg rounded-tr-lg rounded-bl-none bg-background-secondary p-2 py-3 lg:flex-1 lg:rounded-tr-none lg:rounded-bl-lg lg:pr-4">
          {/* Row 1: Type + Name (always together) */}
          <div className="flex flex-wrap items-center gap-2">
            <div
              className={cn(
                "inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-medium",
                getBackgroundColor(deployment.deploymentType),
              )}
            >
              <DeploymentIcon deployment={deployment} className="size-3.5" />
              <span>{getDeploymentTypeLabel(deployment.deploymentType)}</span>
            </div>
            <div className="font-mono text-sm text-content-primary">
              {deployment.name}
            </div>
          </div>

          {/* Row 2: Region/Port + Version */}
          <div className="flex flex-wrap items-center gap-6">
            {/* Region (cloud) or Port (local) */}
            {deployment.kind === "cloud" && (
              <div className="flex items-center gap-2">
                <Tooltip tip="Deployment region">
                  <GlobeIcon
                    className="size-4 shrink-0 text-content-secondary"
                    aria-label="Region"
                  />
                </Tooltip>
                <div className="text-sm text-content-primary">
                  {regionDisplayName}
                </div>
              </div>
            )}
            {deployment.kind === "local" && (
              <div className="flex items-center gap-2">
                <Tooltip tip="Local port">
                  <CodeIcon
                    className="size-4 shrink-0 text-content-secondary"
                    aria-label="Port"
                  />
                </Tooltip>
                <div className="text-sm text-content-primary">
                  Port {deployment.port}
                </div>
              </div>
            )}

            {/* Convex Version */}
            {lastPushEvent && (
              <div className="flex items-center gap-2">
                <Tooltip tip="Convex package version">
                  <CubeIcon
                    className="size-4 shrink-0 text-content-secondary"
                    aria-label="Convex package version"
                  />
                </Tooltip>
                <div className="flex items-center">
                  <span className="text-sm text-content-primary">
                    Convex {serverVersion}
                  </span>
                  {hasUpdate && (
                    <Button
                      tip={`A ${
                        serverVersion && latestVersion
                          ? latestVersion.split(".")[0] !==
                            serverVersion.split(".")[0]
                            ? "major"
                            : latestVersion.split(".")[1] !==
                                serverVersion.split(".")[1]
                              ? "minor"
                              : "patch"
                          : ""
                      } update is available for Convex (${serverVersion} → ${latestVersion})`}
                      size="xs"
                      inline
                      aria-label="Convex NPM Package Upgrade Available"
                      href="https://github.com/get-convex/convex-js/blob/main/CHANGELOG.md#changelog"
                      target="_blank"
                      className="h-[1.25rem] text-content-link"
                    >
                      <div>({latestVersion} available)</div>
                    </Button>
                  )}
                </div>
              </div>
            )}
          </div>

          {/* Row 3: Last Deployed + Last Backup */}
          <div className="flex flex-wrap items-center gap-6 gap-y-4">
            {/* Last Deployed */}
            <div className="flex items-center gap-2">
              <Tooltip tip="Last deployment">
                <RocketIcon
                  className="size-4 shrink-0 text-content-secondary"
                  aria-label="Last deployment"
                />
              </Tooltip>
              {!lastPushEvent ? (
                <span className="text-sm text-content-secondary">
                  Never deployed
                </span>
              ) : (
                <div className="flex flex-wrap items-center gap-1 text-sm text-content-primary">
                  <span>Last deployed</span>
                  <TimestampDistance
                    date={new Date(lastPushEvent._creationTime)}
                    className="text-sm text-content-primary"
                  />
                  {creatorId && creatorName && (
                    <>
                      <span>by</span>
                      <TeamMemberLink memberId={creatorId} name={creatorName} />
                    </>
                  )}
                </div>
              )}
            </div>

            {/* Last Backup (only for cloud deployments) */}
            {deployment.kind === "cloud" && (
              <div className="flex items-center gap-2">
                <Tooltip tip="Last backup">
                  <ArchiveIcon
                    className="size-4 shrink-0 text-content-secondary"
                    aria-label="Last backup"
                  />
                </Tooltip>
                <div className="flex items-center gap-1">
                  {lastBackupTime === null ? (
                    <>
                      <span className="text-sm text-content-secondary">
                        No backup yet
                      </span>
                      <Link
                        href={backupSettingsUrl}
                        className="text-content-link hover:underline"
                        aria-label="View backup settings"
                      >
                        <ExternalLinkIcon className="size-3.5" />
                      </Link>
                    </>
                  ) : (
                    <>
                      <span className="text-sm text-content-primary">
                        Last backup created{" "}
                      </span>
                      <TimestampDistance
                        date={new Date(lastBackupTime!)}
                        className="text-sm text-content-primary"
                      />
                      <Link
                        href={backupSettingsUrl}
                        className="text-content-link hover:underline"
                        aria-label="View backup settings"
                      >
                        <ExternalLinkIcon className="size-3.5" />
                      </Link>
                    </>
                  )}
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Deployment URLs */}
        {deployment.kind === "cloud" && (
          <div className="flex flex-col justify-center gap-4 rounded-b-lg border-t bg-background-secondary/70 p-2 py-4 lg:rounded-r-lg lg:rounded-bl-none lg:border-t-0 lg:border-l lg:py-3 lg:pl-4">
            <div className="flex flex-col gap-1">
              <span className="text-xs font-medium text-content-secondary">
                Cloud URL
              </span>
              <Link
                href={convexCloudUrl!}
                target="_blank"
                rel="noopener noreferrer"
                className="font-mono text-xs break-all text-content-link hover:underline"
              >
                {convexCloudUrl}
              </Link>
            </div>
            <div className="flex flex-col gap-1">
              <span className="text-xs font-medium text-content-secondary">
                HTTP Actions URL
              </span>
              <Link
                href={convexSiteUrl!}
                target="_blank"
                rel="noopener noreferrer"
                className="font-mono text-xs break-all text-content-link hover:underline"
              >
                {convexSiteUrl}
              </Link>
            </div>
          </div>
        )}
      </div>
    </Sheet>
  );
}
