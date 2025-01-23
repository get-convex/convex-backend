import { InfiniteScrollList, ModuleFunction } from "dashboard-common";
import { ScheduledJob } from "system-udfs/convex/_system/frontend/common";
import { NoScheduledJobs, NoScheduledJobsForFunction } from "./emptyStates";
import {
  JOB_ITEM_SIZE,
  ScheduledFunctionsListItem,
} from "./ScheduledFunctionsListItem";
import { SCHEDULED_JOBS_PAGE_SIZE } from "./usePaginatedScheduledJobs";

export function ScheduledFunctionsList({
  hasScheduledJobs,
  currentOpenFunction,
  jobs,
  outerRef,
  status,
  loadMore,
  isPaused,
}: {
  hasScheduledJobs: boolean;
  currentOpenFunction: ModuleFunction | undefined;
  jobs: ScheduledJob[];
  outerRef: React.RefObject<HTMLElement>;
  status: "LoadingFirstPage" | "LoadingMore" | "Exhausted" | "CanLoadMore";
  loadMore: (numItems: number) => void;
  isPaused: boolean;
}) {
  return hasScheduledJobs && !currentOpenFunction ? (
    <NoScheduledJobs />
  ) : hasScheduledJobs ? (
    <NoScheduledJobsForFunction />
  ) : (
    <div className="grow">
      <InfiniteScrollList
        outerRef={outerRef}
        items={jobs}
        // Since the result is paginated, we do not know the total number of items.
        totalNumItems={
          (isPaused && jobs.length) || status === "Exhausted"
            ? jobs.length
            : jobs.length + 1
        }
        itemKey={(idx, job) => job.jobs[idx]?._id || ""}
        itemSize={JOB_ITEM_SIZE}
        itemData={{ jobs }}
        pageSize={SCHEDULED_JOBS_PAGE_SIZE}
        RowOrLoading={ScheduledFunctionsListItem}
        loadMore={isPaused ? undefined : loadMore}
      />
    </div>
  );
}
