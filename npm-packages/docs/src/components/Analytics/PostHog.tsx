import { useEffect } from "react";
import { useAnalyticsCookies } from "./useAnalyticsCookies";
import posthog from "posthog-js";
import useDocusaurusContext from "@docusaurus/useDocusaurusContext";

declare global {
  interface Window {
    kapaSettings?: {
      user: {
        uniqueClientId: string;
        email?: string;
      };
    };
  }
}

export default function PostHog() {
  const { allowsCookies } = useAnalyticsCookies();
  const { siteConfig } = useDocusaurusContext();

  useEffect(() => {
    // Note that this the the 'Project API Key' from PostHog, which is
    // write-only and PostHog says is safe to use in public apps.
    const key = siteConfig.customFields.POST_HOG_KEY as string;
    const api_host = siteConfig.customFields.POST_HOG_HOST as string;
    // Note that this is a production build, which includes deploy previews.
    const isProduction = siteConfig.customFields.NODE_ENV === "production";

    if (!isProduction || !key || !api_host) {
      return;
    }

    // See https://posthog.com/docs/libraries/js#config
    posthog.init(key, {
      api_host,
      ui_host: "https://us.posthog.com/",
      // Set to true to log PostHog events to the console.
      debug: false,
      // We capture pageviews manually within analyticsModule.ts.
      capture_pageview: false,
      // By default, we use 'cookieless' tracking:
      // https://posthog.com/tutorials/cookieless-tracking
      persistence: "memory",
    });

    // Identifies this user to Kapa, using their Convex ID if they've signed
    // into the dashboard, and their anonymouse PostHog ID if not. Have
    // confirmed with Kapa that changing the ID (if the user later signs in)
    // will update the existing profile rather than creating a new one.
    const distinctId = posthog.get_distinct_id();
    if (distinctId) {
      window.kapaSettings = {
        user: {
          uniqueClientId: distinctId,
        },
      };
    }
  }, [siteConfig]);

  // Update to allow PostHog to set cookies once consent is given.
  useEffect(() => {
    if (allowsCookies) {
      posthog.set_config({
        persistence: "localStorage+cookie",
      });
    }
  }, [allowsCookies]);

  return null;
}
