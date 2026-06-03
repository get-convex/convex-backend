import { Sheet } from "@ui/Sheet";
import { cn } from "@ui/cn";
import { Link } from "@ui/Link";
import { Button } from "@ui/Button";
import { Spinner } from "@ui/Spinner";
import { Tooltip } from "@ui/Tooltip";
import { TimestampDistance } from "@common/elements/TimestampDistance";
import { useLatestConvexVersion } from "@common/features/health/components/DeploymentSummary";
import { deploymentTypeColorClasses } from "@common/lib/deploymentTypeColorClasses";
import udfs from "@common/udfs";
import { useQuery } from "convex/react";
import { CubeIcon, RocketIcon } from "@radix-ui/react-icons";
import { WrenchIcon } from "@heroicons/react/24/outline";

/**
 * Self-hosted analogue of the cloud `<DeploymentSummary>` card.
 *
 * Shows a "Self-Hosted" badge, the backend version (with the same
 * "update available" comparison the cloud uses), the last deployment
 * timestamp, and the deployment + HTTP Actions URLs reported by the
 * backend itself. Self-hosted has no team / project / region / backup
 * concepts, so those rows are omitted.
 */
export function SelfHostedDeploymentSummary() {
  const convexCloudUrl = useQuery(udfs.convexCloudUrl.default, {});
  const convexSiteUrl = useQuery(udfs.convexSiteUrl.default, {});
  const serverVersion = useQuery(udfs.getVersion.default);
  const lastPushEvent = useQuery(udfs.deploymentEvents.lastPushEvent, {});
  const { hasUpdate, latestVersion } = useLatestConvexVersion(
    serverVersion || undefined,
  );

  const isLoading =
    convexCloudUrl === undefined ||
    convexSiteUrl === undefined ||
    serverVersion === undefined ||
    lastPushEvent === undefined;
  const selfHostedDeploymentType = "dev" as const;

  if (isLoading) {
    return (
      <Sheet className="flex w-fit flex-col bg-transparent" padding={false}>
        <div className="flex min-h-30 min-w-lg items-center justify-center rounded-lg bg-background-secondary p-2 py-3">
          <Spinner className="size-8" />
        </div>
      </Sheet>
    );
  }

  return (
    <Sheet className="flex w-fit flex-col bg-transparent" padding={false}>
      <div className="flex flex-col lg:flex-row">
        {/* Main info panel */}
        <div className="flex flex-col gap-4 rounded-l-lg rounded-tr-lg rounded-bl-none bg-background-secondary p-2 py-3 lg:flex-1 lg:rounded-tr-none lg:rounded-bl-lg lg:pr-4">
          {/* Row 1: Self-Hosted badge */}
          <div className="flex flex-wrap items-center gap-2">
            <div
              className={cn(
                "inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-medium",
                // Reuse the dev (green) palette for visual continuity with the
                // cloud screenshot the design copies.
                deploymentTypeColorClasses(selfHostedDeploymentType),
              )}
            >
              <WrenchIcon
                className="size-3.5"
                aria-label="Self-hosted deployment"
              />
              <span>Self-Hosted</span>
            </div>
          </div>

          {/* Row 2: Convex backend version */}
          <div className="flex flex-wrap items-center gap-6">
            <div className="flex items-center gap-2">
              <Tooltip tip="Convex backend version">
                <CubeIcon
                  className="size-4 shrink-0 text-content-secondary"
                  aria-label="Convex backend version"
                />
              </Tooltip>
              <div className="flex items-center">
                <span className="text-sm text-content-primary">
                  Convex {serverVersion}
                </span>
                {hasUpdate && (
                  <Button
                    tip={`A newer Convex release is available (${serverVersion} → ${latestVersion})`}
                    size="xs"
                    inline
                    aria-label="Convex update available"
                    href="https://github.com/get-convex/convex-backend/releases"
                    target="_blank"
                    // eslint-disable-next-line no-restricted-syntax -- manual Link-Button hybrid implementation
                    className="h-5 text-content-link"
                  >
                    <div>({latestVersion} available)</div>
                  </Button>
                )}
              </div>
            </div>
          </div>

          {/* Row 3: Last deployed */}
          <div className="flex flex-wrap items-center gap-6 gap-y-4">
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
                </div>
              )}
            </div>
          </div>
        </div>

        {/* URLs panel */}
        <div className="flex flex-col justify-center gap-4 rounded-b-lg border-t bg-background-secondary/70 p-2 py-4 lg:rounded-r-lg lg:rounded-bl-none lg:border-t-0 lg:border-l lg:py-3 lg:pl-4">
          <div className="flex flex-col gap-1">
            <span className="text-xs font-medium text-content-secondary">
              Deployment URL
            </span>
            <Link
              href={convexCloudUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="font-mono text-xs break-all"
              noUnderline
            >
              {convexCloudUrl}
            </Link>
          </div>
          <div className="flex flex-col gap-1">
            <span className="text-xs font-medium text-content-secondary">
              HTTP Actions URL
            </span>
            <Link
              href={convexSiteUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="font-mono text-xs break-all"
              noUnderline
            >
              {convexSiteUrl}
            </Link>
          </div>
        </div>
      </div>
    </Sheet>
  );
}
