import type { NextApiRequest, NextApiResponse } from "next";

export default function handler(req: NextApiRequest, res: NextApiResponse) {
  const protocol = req.headers["x-forwarded-proto"] || "https";
  const host = req.headers["x-forwarded-host"] || req.headers.host;
  const baseUrl = `${protocol}://${host}`;
  const clientId = `${baseUrl}/api/posthog-oauth-client`;
  const redirectUri = `${baseUrl}/oauth/posthog/callback`;

  res.setHeader("Cache-Control", "public, max-age=300");
  res.status(200).json({
    client_id: clientId,
    client_name: "Convex Dashboard",
    client_uri: baseUrl,
    redirect_uris: [redirectUri],
    grant_types: ["authorization_code"],
    response_types: ["code"],
    token_endpoint_auth_method: "none",
    application_type: "web",
    scope: "openid project:read user:read",
  });
}
