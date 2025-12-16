import useSWR from "swr";
import type {
  ConvexStatus,
  ConvexStatusIndicator,
} from "lib/ConvexStatusWidget";

export type { ConvexStatus, ConvexStatusIndicator };

interface ConvexStatusResponse {
  status: {
    indicator: ConvexStatusIndicator;
    description: string;
  };
}

/**
 * Hook to poll the Convex status page API and get current status information.
 * Polls every 30 seconds and on window focus (throttled to 30 seconds).
 */
export function useConvexStatus(): {
  status: ConvexStatus | undefined;
} {
  const { data } = useSWR<ConvexStatusResponse>("/api/status", {
    refreshInterval: 1000 * 30,
    focusThrottleInterval: 1000 * 30,
    shouldRetryOnError: false,
    fetcher: convexStatusFetcher,
  });

  return {
    status: data?.status,
  };
}

const convexStatusFetcher = async (
  url: string,
): Promise<ConvexStatusResponse> => {
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error("Failed to fetch Convex status information.");
  }
  return res.json();
};
