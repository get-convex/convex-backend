// Bridge: validates the user's BetterAuth session, then calls the
// orchestrator's `/api/internal/exchange_session` (service-key authenticated)
// to mint a PAT scoped to that user. The PAT is returned to the client and
// used in the Authorization header for orchestrator API calls.

import type { NextApiRequest, NextApiResponse } from "next";
import { auth } from "../../../lib/auth";

const ORCHESTRATOR_URL =
  process.env.CONVEX_ORCHESTRATOR_URL ??
  process.env.PUBLIC_ORCHESTRATOR_URL ??
  "http://localhost:8050";

const SERVICE_KEY = process.env.CONVEX_ORCHESTRATOR_SERVICE_KEY;

export default async function handler(
  req: NextApiRequest,
  res: NextApiResponse,
) {
  if (req.method !== "GET" && req.method !== "POST") {
    res.setHeader("Allow", "GET, POST");
    res.status(405).json({ error: "method not allowed" });
    return;
  }
  if (!SERVICE_KEY) {
    res.status(503).json({
      error:
        "CONVEX_ORCHESTRATOR_SERVICE_KEY is not configured on the dashboard",
    });
    return;
  }

  const requestHeaders: [string, string][] = Object.entries(
    req.headers,
  ).flatMap(([k, v]) =>
    Array.isArray(v)
      ? v.map((x): [string, string] => [k, x])
      : v
        ? [[k, v]]
        : [],
  );

  const session = await auth.api.getSession({
    headers: new Headers(requestHeaders),
  });
  if (!session?.user) {
    res.status(401).json({ error: "not authenticated" });
    return;
  }
  const inviteCode =
    typeof req.query.inviteCode === "string" ? req.query.inviteCode : undefined;

  const exchangeRes = await fetch(
    `${ORCHESTRATOR_URL.replace(/\/$/, "")}/api/internal/exchange_session`,
    {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-Service-Key": SERVICE_KEY,
      },
      body: JSON.stringify({
        authUserId: session.user.id,
        email: session.user.email,
        name: session.user.name ?? null,
        ...(inviteCode ? { inviteCode } : {}),
      }),
    },
  );

  if (!exchangeRes.ok) {
    const body = await exchangeRes.text().catch(() => "");
    const status =
      exchangeRes.status === 401 || exchangeRes.status === 403
        ? exchangeRes.status
        : 502;
    res.status(status).json({
      error: "orchestrator session exchange failed",
      status: exchangeRes.status,
      body,
    });
    return;
  }

  const data = (await exchangeRes.json()) as {
    accessToken: string;
    memberId: number;
    teamSlug: string;
    role: string;
  };

  // Headers to discourage caching of bearer tokens.
  res.setHeader("Cache-Control", "no-store, max-age=0");
  res.status(200).json(data);
}
