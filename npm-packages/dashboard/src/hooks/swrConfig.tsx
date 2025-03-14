import { SWRConfiguration } from "swr";
import { backoffWithJitter } from "@common/lib/utils";
import { bigBrainAuth } from "hooks/fetching";
import { checkMutex } from "api/onlineStatus";
import {
  TOAST_AFTER_BACKOFF_COUNT,
  showOfflineToast,
} from "api/offlineNotification";

// defaults set for big brain, instances APIs need to explicitly use the other fetcher.
export const swrConfig = (): SWRConfiguration => ({
  use: [bigBrainAuth],
  onErrorRetry: (error, _key, _config, revalidate, { retryCount }) => {
    if (error.status === 404) {
      return;
    }
    // If an error instance made it through to
    // this handler, it's an error that we didn't
    // handle in the fetching layer. This happens for
    // deployment-related fetch errors.
    if (error instanceof Error) {
      // Show toast after certain number of retries
      if (retryCount === TOAST_AFTER_BACKOFF_COUNT) {
        showOfflineToast(error);
      }
    }

    const nextBackoff = backoffWithJitter(retryCount);
    setTimeout(
      () =>
        checkMutex
          ?.then((isOnline) => {
            isOnline && void revalidate({ retryCount });
          })
          .catch(() => {
            // Ignore but handle just in case
          }),
      nextBackoff,
    );
  },
});
