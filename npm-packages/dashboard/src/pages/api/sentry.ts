import { wrapApiHandlerWithSentry, captureException } from "@sentry/nextjs";
import type { NextApiRequest, NextApiResponse } from "next";

// Change host appropriately if you run your own Sentry instance.
const sentryHost = "o1192621.ingest.sentry.io";

// Set knownProjectIds to an array with your Sentry project IDs which you
// want to accept through this proxy.
const knownProjectIds = ["6348388"];

async function handler(req: NextApiRequest, res: NextApiResponse) {
  try {
    const envelope = req.body;
    const pieces = envelope.split("\n");

    const header = JSON.parse(pieces[0]);

    const { pathname } = new URL(header.dsn);

    const projectId = pathname.endsWith("/")
      ? pathname.slice(1, -1)
      : pathname.slice(1);
    if (!knownProjectIds.includes(projectId)) {
      throw new Error(`invalid project id: ${projectId}`);
    }

    const url = `https://${sentryHost}/api/${projectId}/envelope/`;
    const response = await fetch(url, {
      method: "POST",
      body: envelope,
    });
    res.status(200).json(response.json());
  } catch (e) {
    captureException(e);
    res.status(400).json({ status: "invalid request" });
  }
}

export default wrapApiHandlerWithSentry(handler, "/api/sentry");
export const config = {
  api: {
    externalResolver: true,
  },
};
