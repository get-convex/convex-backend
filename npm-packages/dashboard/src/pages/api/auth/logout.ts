import { NextApiRequest, NextApiResponse } from "next";
import {
  loadSealedSessionFromRequest,
  deleteSessionCookie,
} from "server/workos";

export default async function handler(
  req: NextApiRequest,
  res: NextApiResponse,
) {
  if (req.method !== "GET" && req.method !== "POST") {
    return res.status(405).json({ error: "Method not allowed" });
  }

  try {
    const session = loadSealedSessionFromRequest(req);

    if (!session) {
      return res.status(401).json({ error: "No session found" });
    }

    const logoutUrl = await session.getLogoutUrl({
      returnTo: `${
        process.env.WORKOS_REDIRECT_URI ||
        `https://${process.env.WORKOS_REDIRECT_URI_OVERRIDE}`
      }/login`,
    });

    res.setHeader("Set-Cookie", deleteSessionCookie());
    res.redirect(logoutUrl);
  } catch (error) {
    console.error("Error during logout:", error);
    res.status(500).json({ error: "Internal server error" });
  }
}
