import React from "react";
import { useSize } from "react-use";
import { cn } from "lib/cn";
import { SchedulerStatus } from "../../../elements/SchedulerStatus";
import { FailureRate } from "./FailureRate";
import { CacheHitRate } from "./CacheHitRate";
import { ExceptionReporting } from "./ExceptionReporting";
import { LogStreams } from "./LogStreams";
import { LastDeployed } from "./LastDeployed";

export function Health({
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

  return (
    <div className="flex max-h-full max-w-full flex-col gap-2 overflow-hidden pb-4 pt-2">
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
