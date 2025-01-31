import { useRef } from "react";
import { usePaginatedScheduledJobs } from "features/schedules/lib/usePaginatedScheduledJobs";
import { ScheduledFunctionsContentToolbar } from "features/schedules/components/ScheduledFunctionsContentToolbar";
import { ScheduledFunctionsListHeader } from "features/schedules/components/ScheduledFunctionsListHeader";
import { ScheduledFunctionsList } from "features/schedules/components/ScheduledFunctionsList";
import { Sheet } from "elements/Sheet";
import { ModuleFunction } from "lib/functions/types";

export function ScheduledFunctionsContent({
  currentOpenFunction,
}: {
  currentOpenFunction: ModuleFunction | undefined;
}) {
  const {
    jobs,
    status,
    loadMore,
    isRateLimited,
    togglePaused,
    reload,
    isPaused,
    isLoadingPausedData,
  } = usePaginatedScheduledJobs(currentOpenFunction?.identifier);

  const outerRef = useRef<HTMLElement>(null);

  const isDataLoaded = isPaused || status !== "LoadingFirstPage";

  const hasScheduledJobs = jobs.length === 0 && status === "Exhausted";

  return (
    <div className="relative flex h-full max-w-6xl grow flex-col gap-4">
      <ScheduledFunctionsContentToolbar jobs={jobs} />
      <Sheet
        className="flex min-w-[40rem] max-w-full grow flex-col"
        padding={false}
      >
        <ScheduledFunctionsListHeader
          isPaused={isPaused}
          isLoadingPausedData={isLoadingPausedData}
          togglePaused={togglePaused}
          isRateLimited={isRateLimited}
          reload={reload}
        />
        {isDataLoaded && (
          <ScheduledFunctionsList
            hasScheduledJobs={hasScheduledJobs}
            currentOpenFunction={currentOpenFunction ?? undefined}
            jobs={jobs}
            outerRef={outerRef}
            status={status}
            loadMore={loadMore}
            isPaused={isPaused}
          />
        )}
      </Sheet>
    </div>
  );
}
