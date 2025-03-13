// This file configures the initialization of Sentry on the browser.
// The config you add here will be used whenever a page is visited.
// https://docs.sentry.io/platforms/javascript/guides/nextjs/

import * as Sentry from "@sentry/nextjs";

const SENTRY_DSN = process.env.SENTRY_DSN || process.env.NEXT_PUBLIC_SENTRY_DSN;
const environment =
  process.env.NEXT_PUBLIC_ENVIRONMENT === "production"
    ? "production"
    : "development";

Sentry.init({
  dsn: SENTRY_DSN,
  profilesSampleRate: 0.5,
  tracesSampleRate: 0.5,
  tunnel: "/api/sentry",
  environment,
  integrations: [
    new Sentry.BrowserTracing({
      tracingOrigins: ["localhost", /^\//, /.*\.convex.cloud/],
    }),
    new Sentry.Replay({ useCompression: false }),
  ],
  replaysSessionSampleRate: 0.1,
  replaysOnErrorSampleRate: 1.0,
  release: process.env.NEXT_PUBLIC_VERCEL_GIT_COMMIT_SHA,
  ignoreErrors: [
    "ResizeObserver loop completed with undelivered notifications.",
    "ConvexReactClient has already been closed.",
    /.*AccessTokenInvalid.*/,
  ],
});
