import { useQuery } from "convex/react";
import udfs from "@common/udfs";
import { cn } from "@common/lib/cn";
import { HealthCard } from "@common/elements/HealthCard";
import { TimestampDistance } from "@common/elements/TimestampDistance";
import { Loading } from "@common/elements/Loading";
import { useEffect, useState } from "react";
import semver from "semver";
import { Button } from "@common/elements/Button";

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
          const hasNewVersion = semver.gt(data.version, currentVersion);
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
        <div className="flex items-center gap-2">
          <span className="animate-fadeInFromLoading text-sm text-content-secondary">
            Convex v{serverVersion}
          </span>
          {hasUpdate && (
            <Button
              tip={`A ${
                serverVersion && latestVersion
                  ? latestVersion.split(".")[0] !== serverVersion.split(".")[0]
                    ? "major"
                    : latestVersion.split(".")[1] !==
                        serverVersion.split(".")[1]
                      ? "minor"
                      : "patch"
                  : ""
              } update is available for Convex (${serverVersion} → ${latestVersion})`}
              className="bg-util-accent p-0.5 px-1 text-white"
              href="https://www.npmjs.com/package/convex?activeTab=versions"
              target="_blank"
            >
              Update Available
            </Button>
          )}
        </div>
      </div>
    </HealthCard>
  );
}
