import { NextApiRequest, NextApiResponse } from "next";
import { WorkOS } from "@workos-inc/node";
import { captureException } from "@sentry/nextjs";

export default async function handler(
  req: NextApiRequest,
  res: NextApiResponse,
) {
  if (req.method !== "GET") {
    return res.status(405).json({ error: "Method not allowed" });
  }
  const workos = new WorkOS(process.env.WORKOS_API_SECRET, {
    clientId: process.env.WORKOS_CLIENT_ID,
  });

  // The authorization code returned by AuthKit
  const code = req.query.code as string;
  const state = req.query.state as string; // This contains our returnTo URL
  const returnTo = state && !state.startsWith("/api") ? state : "/";

  if (!code) {
    return res.status(400).send("No code provided");
  }

  try {
    const authenticateResponse =
      await workos.userManagement.authenticateWithCode({
        clientId: process.env.WORKOS_CLIENT_ID || "",
        code,
        session: {
          sealSession: true,
          cookiePassword: process.env.WORKOS_COOKIE_PASSWORD,
        },
      });

    const { sealedSession } = authenticateResponse;

    // Store the session in a cookie
    const secure =
      // We only use secure cookies in production because development environments might use HTTP
      // (most browsers tolerate secure cookies on localhost, but not Safari)
      process.env.NODE_ENV === "production" ? " Secure;" : "";
    res.setHeader(
      "Set-Cookie",
      `wos-session=${sealedSession}; Path=/; HttpOnly;${secure} SameSite=Lax`,
    );

    // Use the information in `user` for further business logic.

    // Redirect the user to the homepage
    res.redirect(returnTo);
  } catch (error: any) {
    if (
      error.status === 403 &&
      error.rawData?.code === "email_verification_required"
    ) {
      res.redirect(
        `${process.env.WORKOS_LOGIN_URL}/email-verification?` +
          `email=${error.rawData.email}&` +
          `pending_authentication_token=${error.rawData?.pending_authentication_token}&` +
          `state=${state}&` +
          `redirect_uri=${state || "/"}`,
      );
      return;
    }
    captureException(error);
    // TODO: Figure out what to do here.
    // res.redirect(
    //   `/login?error=${error?.rawData?.error_description || error.message}`,
    // );
  }
}
