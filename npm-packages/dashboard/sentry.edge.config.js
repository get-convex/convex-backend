// sentry.edge.config.js or sentry.edge.config.ts

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
  tracesSampleRate: 0.1,
  tunnel: "/api/sentry",
  environment,
  integrations: [
    new RequestData({
      include: {
        cookies: false,
      },
    }),
  ],
});
