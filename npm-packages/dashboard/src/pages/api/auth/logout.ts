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

  const returnTo =
    typeof req.query.returnTo === "string" &&
    req.query.returnTo.startsWith("/") &&
    !req.query.returnTo.startsWith("//")
      ? req.query.returnTo
      : "/login";

  const sessionDeleted = req.query.sessionDeleted === "true";

  try {
    const session = loadSealedSessionFromRequest(req);

    res.setHeader("Set-Cookie", deleteSessionCookie());

    if (!session || sessionDeleted) {
      return res.redirect(returnTo);
    }

    const logoutUrl = await session.getLogoutUrl({
      returnTo: `${
        process.env.WORKOS_REDIRECT_URI ||
        `https://${process.env.WORKOS_REDIRECT_URI_OVERRIDE}`
      }/login`,
    });

    res.redirect(logoutUrl);
  } catch (error) {
    console.error("Error during logout:", error);
    res.redirect(returnTo);
  }
}
