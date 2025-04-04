import React from "react";
import { useSize } from "react-use";
import { cn } from "@common/lib/cn";
import { SchedulerStatus } from "@common/elements/SchedulerStatus";
import { FailureRate } from "@common/features/health/components/FailureRate";
import { CacheHitRate } from "@common/features/health/components/CacheHitRate";
import { ExceptionReporting } from "@common/features/health/components/ExceptionReporting";
import { LogStreams } from "@common/features/health/components/LogStreams";
import { LastDeployed } from "@common/features/health/components/LastDeployed";
import { DeploymentPageTitle } from "@common/elements/DeploymentPageTitle";
import { PageContent } from "@common/elements/PageContent";
import udfs from "@common/udfs";
import { useQuery } from "convex/react";
import { Sheet } from "@common/elements/Sheet";
import { EmptySection } from "@common/elements/EmptySection";
import Image from "next/image";

export function HealthView({
  header,
  PageWrapper,
  PagesWrapper,
}: {
  header: JSX.Element;
  PageWrapper: React.FC<{ children: React.ReactNode }>;
  PagesWrapper: React.FC<{ children: React.ReactNode }>;
}) {
  const [sizedHeader, { width }] = useSize(header);

  const gridClasses =
    width > 1280
      ? "grid-cols-3 grid"
      : width > 720
        ? "grid-cols-2 grid"
        : "flex flex-col items-center";

  const lastPushEvent = useQuery(udfs.deploymentEvents.lastPushEvent, {});
  return (
    <PageContent>
      <DeploymentPageTitle title="Health" />
      {lastPushEvent === null && (
        <div className="absolute z-50 flex h-full w-full max-w-[88rem] items-center justify-center overflow-hidden bg-[black]/10">
          <Sheet className="m-6 h-fit w-fit max-w-lg bg-background-secondary/95 p-2 backdrop-blur-[2px]">
            <EmptySection
              sheet={false}
              color="none"
              Icon={() => (
                <Image
                  src="/convex-logo-only.svg"
                  width="28"
                  height="28"
                  alt="Convex logo"
                />
              )}
              header="Welcome to the Convex Dashboard!"
              body="Information about your deployment will appear here once you deploy your first function."
              learnMoreButton={{
                href: "https://docs.convex.dev/quickstarts",
                children: "Follow a quickstart guide",
              }}
            />
          </Sheet>
        </div>
      )}
      <div className="flex max-h-full max-w-full flex-col gap-2 overflow-hidden pt-2">
        {sizedHeader}
        <PagesWrapper>
          <PageWrapper>
            <div className={cn(gridClasses, "gap-4 max-w-[88rem]")}>
              <MetricCards />
              <SchedulerStatus />
              {width <= 1280 ? (
                <div className="flex w-full min-w-48 flex-col justify-between gap-4">
                  <LastDeployed />
                  <ExceptionReporting />
                  <LogStreams />
                </div>
              ) : (
                <>
                  <LastDeployed />
                  <ExceptionReporting />
                  <LogStreams />
                </>
              )}
            </div>
          </PageWrapper>
        </PagesWrapper>
      </div>
    </PageContent>
  );
}

function MetricCards() {
  return (
    <>
      <FailureRate />
      <CacheHitRate />
    </>
  );
}
