import { useProfile } from "api/profile";
import { Router } from "next/router";
import posthog from "posthog-js";
import { PostHogProvider as PHProvider } from "posthog-js/react";
import React, { useEffect, useState } from "react";

export function PostHogProvider({
  children,
}: {
  children: React.ReactElement;
}) {
  const profile = useProfile();
  const [postHogReady, setPostHogReady] = useState(() => posthog.__loaded);

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
      loaded() {
        setPostHogReady(true);
      },
    });

    // Capture pageview events on route change.
    const handleRouteChange = () => posthog?.capture("$pageview");
    Router.events.on("routeChangeComplete", handleRouteChange);

    return () => {
      Router.events.off("routeChangeComplete", handleRouteChange);
    };
  }, []);

  // We wait for PostHog so it can read the shared subdomain cookie first, which
  // may contain an anonymous ID from an earlier visit to the website, docs,
  // Stack, etc. Calling identify() before that loads would skip the merge and
  // we'd lose $anon_distinct_id on this person.
  useEffect(() => {
    if (!profile || !postHogReady) {
      return;
    }

    const profileId = profile.id.toString();
    const postHogId = posthog.get_distinct_id();
    if (postHogId === profileId) {
      // This user has already been identified.
      return;
    }

    posthog.identify(profileId);
  }, [profile, postHogReady]);

  return <PHProvider client={posthog}>{children}</PHProvider>;
}
