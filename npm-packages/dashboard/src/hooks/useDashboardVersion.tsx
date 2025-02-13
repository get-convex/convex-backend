import useSWR from "swr";

import { toast } from "dashboard-common/lib/utils";
import { Button } from "dashboard-common/elements/Button";
import { SymbolIcon } from "@radix-ui/react-icons";
import { captureMessage } from "@sentry/nextjs";
import { LocalDevCallout } from "dashboard-common/elements/Callout";

// To test that this works
// set the following in your .env.local:
// NEXT_PUBLIC_VERCEL_GIT_COMMIT_SHA=<SHA_THAT_ISN'T_THE_LATEST>
// VERCEL_TOKEN=<VERCEL_ACCESS_TOKEN>
export function useDashboardVersion() {
  const { data, error } = useSWR<{ sha?: string | null }>("/api/version", {
    // Refresh every hour.
    refreshInterval: 1000 * 60 * 60,
    // Refresh on focus at most every 10 minutes.
    focusThrottleInterval: 1000 * 60 * 10,
    shouldRetryOnError: false,
    fetcher: dashboardVersionFetcher,
  });

  const currentSha = process.env.NEXT_PUBLIC_VERCEL_GIT_COMMIT_SHA;
  if (!error && data?.sha && currentSha && data?.sha !== currentSha) {
    toast(
      "info",
      <div className="flex flex-col">
        A new version of the Convex dashboard is available! Refresh this page to
        update.
        <LocalDevCallout tipText="In local development, the local git sha is being compared to the latest production deployment's sha." />
        <Button
          className="ml-auto w-fit items-center"
          inline
          size="xs"
          icon={<SymbolIcon />}
          // Make the href the current page so that the page refreshes.
          onClick={() => window.location.reload()}
        >
          Refresh
        </Button>
      </div>,
      "dashboardVersion",
      false,
    );
  }
}

// Custom fetcher because we're using Vercel functions and not big brain.
const dashboardVersionFetcher = async (url: string) => {
  const res = await fetch(url);
  if (!res.ok) {
    try {
      const { error } = await res.json();
      captureMessage(error);
    } catch (e) {
      captureMessage("Failed to fetch dashboard version information.");
    }
    throw new Error("Failed to fetch dashboard version information.");
  }
  return res.json();
};
