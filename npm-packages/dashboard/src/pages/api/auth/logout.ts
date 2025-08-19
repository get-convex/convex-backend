import { NextApiRequest, NextApiResponse } from "next";
import { WorkOS } from "@workos-inc/node";

export default async function handler(
  req: NextApiRequest,
  res: NextApiResponse,
) {
  if (req.method !== "GET" && req.method !== "POST") {
    return res.status(405).json({ error: "Method not allowed" });
  }

  const workos = new WorkOS(process.env.WORKOS_API_SECRET, {
    clientId: process.env.WORKOS_CLIENT_ID,
  });

  try {
    const cookieHeader = req.headers.cookie;
    if (!cookieHeader) {
      return null;
    }
    const sessionCookie = cookieHeader
      .split(";")
      .find((cookie) => cookie.trim().startsWith("wos-session="))
      ?.split("=")[1];

    if (!sessionCookie) {
      return null;
    }

    // Verify and unseal the session
    const session = workos.userManagement.loadSealedSession({
      sessionData: sessionCookie,
      cookiePassword: process.env.WORKOS_COOKIE_PASSWORD || "",
    });
    const logoutUrl = await session.getLogoutUrl({
      returnTo: `${
        process.env.WORKOS_REDIRECT_URI ||
        `https://${process.env.WORKOS_REDIRECT_URI_OVERRIDE}`
      }/login`,
    });
    res.setHeader("Set-Cookie", [
      "wos-session=deleted; Max-Age=-1; Path=/; HttpOnly; Secure; SameSite=Lax",
    ]);
    res.redirect(logoutUrl);
  } catch (error) {
    console.error("Error during logout:", error);
    res.status(500).json({ error: "Internal server error" });
  }
}
