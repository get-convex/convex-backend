import { Sheet, ModuleFunction } from "dashboard-common";
import { useRef } from "react";
import { usePaginatedScheduledJobs } from "./usePaginatedScheduledJobs";
import { ScheduledFunctionsContentToolbar } from "./ScheduledFunctionsContentToolbar";
import { ScheduledFunctionsListHeader } from "./ScheduledFunctionsListHeader";
import { ScheduledFunctionsList } from "./ScheduledFunctionsList";

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
