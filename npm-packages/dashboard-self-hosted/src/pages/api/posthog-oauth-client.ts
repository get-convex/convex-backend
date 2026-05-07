import type { NextApiRequest, NextApiResponse } from "next";
import { buildPostHogOAuthClientMetadata } from "@common/features/settings/lib/posthogOAuth";

export default function handler(req: NextApiRequest, res: NextApiResponse) {
  const protocol = req.headers["x-forwarded-proto"] || "https";
  const host = req.headers["x-forwarded-host"] || req.headers.host;
  const baseUrl = `${protocol}://${host}`;

  res.setHeader("Cache-Control", "public, max-age=300");
  res.status(200).json(buildPostHogOAuthClientMetadata(baseUrl));
}
