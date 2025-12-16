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
    const isProduction = process.env.NODE_ENV === "production";
    const isDebugMode = process.env.NEXT_PUBLIC_POSTHOG_DEBUG === "true";
    // Public 'Project API Key' for PostHog, safe to expose.
    const key = process.env.NEXT_PUBLIC_POSTHOG_KEY;
    const api_host = process.env.NEXT_PUBLIC_POSTHOG_HOST;

    const shouldInitialize =
      (isProduction || isDebugMode) && key && api_host && !posthog.__loaded;

    if (!shouldInitialize) {
      return;
    }

    // See https://posthog.com/docs/libraries/js#config
    posthog.init(key, {
      api_host,
      ui_host: "https://us.posthog.com/",
      // Logs event details to the console.
      debug: isDebugMode,
      // Since we're using the pages router, this captures the initial pageview.
      capture_pageview: true,
      session_recording: {
        recordHeaders: false,
        maskTextSelector: "*", // Masks all text elements (not including inputs)
        maskAllInputs: true, // Masks all input elements
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
