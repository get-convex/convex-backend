// This file configures the initialization of Sentry on the server.
// The config you add here will be used whenever the server handles a request.
// https://docs.sentry.io/platforms/javascript/guides/nextjs/

import * as Sentry from "@sentry/nextjs";

import { Integrations } from "@sentry/nextjs";
const { RequestData } = Integrations;

const SENTRY_DSN = process.env.SENTRY_DSN || process.env.NEXT_PUBLIC_SENTRY_DSN;
const environment =
  process.env.NEXT_PUBLIC_ENVIRONMENT === "production"
    ? "production"
    : "development";

Sentry.init({
  dsn: SENTRY_DSN,
  tracesSampleRate: 0.01,
  release: process.env.SENTRY_RELEASE,
  environment,
  integrations: [
    new RequestData({
      include: {
        cookies: false,
      },
    }),
  ],
});
