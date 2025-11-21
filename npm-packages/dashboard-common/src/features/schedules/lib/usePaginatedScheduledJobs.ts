import { usePaginatedQuery_experimental } from "convex/react";
import { useContext } from "react";
import udfs from "@common/udfs";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { useGlobalLocalStorage } from "@common/lib/useGlobalLocalStorage";
import { useNents } from "@common/lib/useNents";
import { usePausedLiveData } from "@common/lib/usePausedLiveData";

export const SCHEDULED_JOBS_PAGE_SIZE = 50;

export function usePaginatedScheduledJobs(udfPath: string | undefined) {
  const { useCurrentDeployment } = useContext(DeploymentInfoContext);
  const deployment = useCurrentDeployment();

  const [isPaused] = useGlobalLocalStorage(
    `${deployment?.name}/pauseLiveScheduledJobs`,
    false,
  );
  const args = {
    udfPath,
    componentId: useNents().selectedNent?.id ?? null,
  };

  const { results, loadMore, status } = usePaginatedQuery_experimental(
    udfs.paginatedScheduledJobs.default,
    // If we're paused, don't show the live query.
    isPaused ? "skip" : args,
    {
      initialNumItems: SCHEDULED_JOBS_PAGE_SIZE,
    },
  );

  const {
    pausedData,
    isLoadingPausedData,
    isRateLimited,
    togglePaused,
    reload,
  } = usePausedLiveData({
    results,
    args,
    storageKey: "pauseLiveScheduledJobs",
    udfName: udfs.paginatedScheduledJobs.default,
    numItems: SCHEDULED_JOBS_PAGE_SIZE,
  });

  return {
    jobs: isPaused ? pausedData : results,
    status: isPaused
      ? isLoadingPausedData
        ? "LoadingFirstPage"
        : "Exhausted"
      : status,
    isPaused,
    isLoadingPausedData,
    isRateLimited,
    loadMore,
    togglePaused,
    reload,
  };
}
