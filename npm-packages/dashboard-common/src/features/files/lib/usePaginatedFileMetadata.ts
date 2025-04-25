import { usePaginatedQuery } from "convex/react";
import { useContext } from "react";
import udfs from "@common/udfs";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { useGlobalLocalStorage } from "@common/lib/useGlobalLocalStorage";
import { useNents } from "@common/lib/useNents";
import { usePausedLiveData } from "@common/lib/usePausedLiveData";

export const FILE_METADATA_PAGE_SIZE = 20;

export function usePaginatedFileMetadata() {
  const { useCurrentDeployment } = useContext(DeploymentInfoContext);
  const deployment = useCurrentDeployment();

  const [isPaused] = useGlobalLocalStorage(
    `${deployment?.name}/pauseLiveFileStorage`,
    false,
  );

  const args = { componentId: useNents().selectedNent?.id ?? null };

  const { results, loadMore, status } = usePaginatedQuery(
    udfs.fileStorageV2.fileMetadata,
    // If we're paused, don't show the live query.
    isPaused ? "skip" : args,
    {
      initialNumItems: FILE_METADATA_PAGE_SIZE,
    },
  );

  const {
    pausedData,
    isLoadingPausedData,
    isRateLimited,
    togglePaused,
    reload,
    loadMorePaused,
    canLoadMore,
    isLoadingMore,
  } = usePausedLiveData({
    results,
    args,
    storageKey: "pauseLiveFileStorage",
    udfName: udfs.fileStorageV2.fileMetadata,
    numItems: FILE_METADATA_PAGE_SIZE,
  });

  return {
    files: isPaused ? pausedData : results,
    status: isPaused
      ? isLoadingPausedData
        ? "LoadingFirstPage"
        : canLoadMore
          ? "CanLoadMore"
          : "Exhausted"
      : status,
    isPaused,
    isLoadingPausedData,
    isLoadingMore,
    isRateLimited,
    loadMore: isPaused ? loadMorePaused : loadMore,
    togglePaused,
    reload,
  };
}
