import { SWRConfiguration } from "swr";
import { captureException } from "@sentry/nextjs";
import { backoffWithJitter, toast } from "dashboard-common/lib/utils";
import Link from "next/link";
import React from "react";
import { bigBrainAuth } from "hooks/fetching";

// 500 + 1000 + 2000 + 4000 + 8000 -> Toast after 15.5s
const TOAST_AFTER_BACKOFF_COUNT = 6;

// defaults set for big brain, instances APIs need to explicitly use the other fetcher.
export const swrConfig = (): SWRConfiguration => ({
  use: [bigBrainAuth],
  onErrorRetry: (error, _key, _config, revalidate, { retryCount }) => {
    if (error.status === 404) {
      return;
    }
    captureException(error);

    if (retryCount === TOAST_AFTER_BACKOFF_COUNT) {
      const content = (
        <p>
          Something seems wrong. The Convex team has been alerted. Check{" "}
          <Link
            href="https://status.convex.dev/"
            className="text-content-link hover:underline"
            target="_blank"
          >
            Convex status
          </Link>{" "}
          for details and updates.
        </p>
      );
      toast("error", content, "check_convex_status");
    }

    const nextBackoff = backoffWithJitter(retryCount);
    setTimeout(() => revalidate({ retryCount }), nextBackoff);
  },
});
