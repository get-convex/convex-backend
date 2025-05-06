import { useQuery } from "convex/react";
import udfs from "@common/udfs";
import { cn } from "@ui/cn";
import { HealthCard } from "@common/elements/HealthCard";
import { TimestampDistance } from "@common/elements/TimestampDistance";
import { Loading } from "@ui/Loading";
import { useEffect, useState } from "react";
import semver from "semver";
import { Button } from "@ui/Button";
import { DoubleArrowUpIcon } from "@radix-ui/react-icons";

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
      } catch (e) {
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

export function LastDeployed() {
  const lastPushEvent = useQuery(udfs.deploymentEvents.lastPushEvent, {});
  const serverVersion = useQuery(udfs.getVersion.default);
  const { hasUpdate, latestVersion } = useLatestConvexVersion(
    serverVersion || undefined,
  );

  const content =
    lastPushEvent === undefined ? (
      <Loading className="h-5 w-24" />
    ) : !lastPushEvent ? (
      <span
        className={cn(
          "text-content-secondary text-sm animate-fadeInFromLoading",
        )}
      >
        Never
      </span>
    ) : (
      <TimestampDistance
        date={new Date(lastPushEvent?._creationTime || 0)}
        className="w-fit animate-fadeInFromLoading text-sm text-content-primary"
      />
    );

  return (
    <HealthCard
      title="Last Deployed"
      size="sm"
      tip="The last time functions were deployed."
    >
      <div className="flex h-full w-full grow flex-wrap justify-between px-2 pb-2">
        {content}
        {serverVersion && (
          <div className="flex h-8 items-center gap-2">
            <span className="animate-fadeInFromLoading text-sm text-content-secondary">
              Convex v{serverVersion}
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
                aria-label="Convex NPM Package Upgrade Available"
                href="https://github.com/get-convex/convex-js/blob/main/CHANGELOG.md#changelog"
                target="_blank"
                icon={<DoubleArrowUpIcon />}
              />
            )}
          </div>
        )}
      </div>
    </HealthCard>
  );
}
