import { NextApiRequest, NextApiResponse } from "next";
import { getSession, createSessionCookie } from "server/workos";

export default async function handler(
  req: NextApiRequest,
  res: NextApiResponse,
) {
  if (req.method !== "GET") {
    return res.status(405).json({ error: "Method not allowed" });
  }

  try {
    const session = await getSession(req);

    if (!session) {
      return res.status(401).json({ error: "Not authenticated" });
    }

    // If the session was refreshed, update the cookie
    if (session.sealedSession) {
      res.setHeader("Set-Cookie", createSessionCookie(session.sealedSession));
    }

    // Don't send sealedSession to the client
    const { sealedSession: _sealedSession, ...sessionData } = session;

    res.status(200).json(sessionData);
  } catch (error) {
    console.error("Error getting session:", error);
    res.status(500).json({ error: "Internal server error" });
  }
}
