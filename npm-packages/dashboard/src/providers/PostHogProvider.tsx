import { Router } from "next/router";
import React, { useEffect } from "react";
import posthog from "posthog-js";
import { PostHogProvider as PHProvider } from "posthog-js/react";
import { useProfile } from "api/profile";

export function PostHogProvider({
  children,
}: {
  children: React.ReactElement;
}) {
  const profile = useProfile();

  useEffect(() => {
    // Only initialize PostHog in production.
    if (process.env.NODE_ENV !== "production") {
      return;
    }

    // Note that this is the 'Project API Key' from PostHog, which is write-only
    // and PostHog says is safe to use in public apps.
    const key = process.env.NEXT_PUBLIC_POSTHOG_KEY;
    const api_host = process.env.NEXT_PUBLIC_POSTHOG_HOST;

    if (!key || !api_host) {
      return;
    }

    // See https://posthog.com/docs/libraries/js#config
    posthog.init(key, {
      api_host,
      ui_host: "https://us.posthog.com/",
      // Set to true to log PostHog events to the console.
      debug: false,
      // Since we're using the pages router, this captures the initial pageview.
      capture_pageview: true,
      session_recording: {
        recordHeaders: false,
        maskTextSelector: "*", // Masks all text elements (not including inputs)
      },
    });

    // Capture pageview events on route change.
    const handleRouteChange = () => posthog?.capture("$pageview");
    Router.events.on("routeChangeComplete", handleRouteChange);

    return () => {
      Router.events.off("routeChangeComplete", handleRouteChange);
    };
  }, []);

  // Identify the user with their profile details.
  useEffect(() => {
    if (profile) {
      posthog?.identify(profile.id.toString());
    }
  }, [profile]);

  return <PHProvider client={posthog}>{children}</PHProvider>;
}
