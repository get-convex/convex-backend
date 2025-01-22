import { usePaginatedQuery } from "convex/react";
import { useCurrentDeployment } from "api/deployments";
import { createConvexAdminClient } from "hooks/deploymentApi";
import { useAuthHeader } from "hooks/fetching";
import { useGlobalLocalStorage, useNents, toast } from "dashboard-common";
import { useEffect, useState, useCallback, useRef, useMemo } from "react";
import { useMount } from "react-use";
import { ScheduledJob } from "system-udfs/convex/_system/frontend/common";
import udfs from "udfs";

export const SCHEDULED_JOBS_PAGE_SIZE = 50;

export function usePaginatedScheduledJobs(udfPath: string | undefined) {
  const deployment = useCurrentDeployment();

  const [isPaused] = useGlobalLocalStorage(
    `${deployment?.name}/pauseLiveScheduledJobs`,
    false,
  );
  const args = {
    udfPath,
    componentId: useNents().selectedNent?.id ?? null,
  };

  const { results, loadMore, status } = usePaginatedQuery(
    udfs.paginatedScheduledJobs.default,
    // If we're paused, don't show the live query.
    isPaused ? "skip" : args,
    {
      initialNumItems: SCHEDULED_JOBS_PAGE_SIZE,
    },
  );

  const {
    pausedData,
    isRateLimited,
    isLoadingPausedData,
    togglePaused,
    reload,
  } = usePausedState(results, args);

  return {
    jobs: isPaused ? pausedData : results,
    status,
    isPaused,
    isLoadingPausedData,
    isRateLimited,
    loadMore,
    togglePaused,
    reload,
  };
}

function usePausedState(
  results: ScheduledJob[],
  args: {
    udfPath: string | undefined;
    componentId: string | null;
  },
) {
  const [pausedData, setPausedData] = useState(results);

  const deployment = useCurrentDeployment();

  // Store the paused state in local storage so it persists across refreshes
  const [isPaused, setIsPaused] = useGlobalLocalStorage(
    `${deployment?.name}/pauseLiveScheduledJobs`,
    false,
  );

  const onRateLimited = () => {
    // When we get rate limited, we should immediately pause.
    setIsPaused(true);
    // Store the current result set from the live query so we can show it when paused.
    setPausedData(results);
    toast(
      "error",
      "There are too many scheduled functions to show live updates. Updates have automatically been paused.",
      "liveUpdatesPaused",
    );
  };

  const isRateLimited = useRateLimitChanges(results, isPaused, onRateLimited);

  const authHeader = useAuthHeader();

  const [isLoadingPausedData, setIsLoadingPausedData] = useState(false);
  const loadFirstPage = useMemo(
    () =>
      loadFirstPageOneShot({
        setIsLoadingPausedData,
        setPausedData,
        deploymentName: deployment?.name,
        authHeader,
        args,
      }),
    [args, authHeader, deployment],
  );

  useMount(() => {
    isPaused && void loadFirstPage();
  });

  return {
    pausedData,
    isLoadingPausedData,
    isRateLimited,
    togglePaused: useCallback(() => {
      setPausedData(results);
      setIsPaused(!isPaused);
    }, [results, setIsPaused, isPaused]),
    reload: loadFirstPage,
  };
}

const RATE_LIMIT_BUCKET_MS = 10 * 1000;
// > 10 updates per second
const RATE_LIMIT_THRESHOLD = 10 * 10;

function useRateLimitChanges(
  items: ScheduledJob[],
  isPaused: boolean,
  onRateLimited: () => void,
) {
  const callCountRef = useRef(0);
  const lastResetRef = useRef(Date.now());

  const [isRateLimited, setIsRateLimited] = useState(false);

  useEffect(() => {
    if (isRateLimited || isPaused) {
      return;
    }

    const now = Date.now();
    if (now - lastResetRef.current > RATE_LIMIT_BUCKET_MS) {
      callCountRef.current = 0;
      lastResetRef.current = now;
    }

    callCountRef.current += 1;

    if (callCountRef.current > RATE_LIMIT_THRESHOLD) {
      setIsRateLimited(true);
      onRateLimited();
    }
  }, [items, isPaused, isRateLimited, onRateLimited]);

  return isRateLimited;
}

const loadFirstPageOneShot =
  ({
    setIsLoadingPausedData,
    setPausedData,
    deploymentName,
    authHeader,
    args,
  }: {
    setIsLoadingPausedData: (value: boolean) => void;
    setPausedData: (value: ScheduledJob[]) => void;
    deploymentName?: string;
    authHeader: string;
    args: {
      udfPath: string | undefined;
      componentId: string | null;
    };
  }) =>
  async () => {
    setIsLoadingPausedData(true);
    try {
      if (!deploymentName) {
        throw new Error("Missing deployment name");
      }
      const { client } = await createConvexAdminClient(
        deploymentName,
        authHeader,
      );

      // Fetch one page
      const result = await client.query(udfs.paginatedScheduledJobs.default, {
        ...args,
        paginationOpts: {
          numItems: SCHEDULED_JOBS_PAGE_SIZE,
          cursor: null,
          id: 0,
        },
      });
      setPausedData(result.page);
    } catch (e) {
      toast(
        "error",
        "Failed to load scheduled functions",
        "loadScheduledFunctions",
      );
    } finally {
      setIsLoadingPausedData(false);
    }
  };
