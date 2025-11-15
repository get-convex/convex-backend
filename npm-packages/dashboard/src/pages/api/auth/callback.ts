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

  const { resource_id, path, url } = req.query;

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

    const { sealedSession, authenticationMethod } = authenticateResponse;

    // Store the session in a cookie
    const secure =
      // We only use secure cookies in production because development environments might use HTTP
      // (most browsers tolerate secure cookies on localhost, but not Safari)
      process.env.NODE_ENV === "production" ? " Secure;" : "";
    res.setHeader(
      "Set-Cookie",
      `wos-session=${sealedSession}; Path=/; HttpOnly;${secure} SameSite=Lax; Max-Age=${60 * 60 * 24 * 14}`,
    );

    let returnTo = state && !state.startsWith("/api") ? state : "/";

    // url is a query parameter that is only set by the Vercel auth flow
    // if it is set, and looks like a redirect to the device-auth flow,
    // we redirect to the device-auth flow.
    if (typeof url === "string" && url.startsWith("https://auth.convex.dev")) {
      returnTo = url;
    } else if (typeof path === "string" || typeof resource_id === "string") {
      const key = typeof path === "string" ? "vercelPath" : "projectId";
      const value = typeof path === "string" ? path : resource_id;
      returnTo = addQueryParam(returnTo, key, value as string);
    }

    // @ts-expect-error VercelOAuth is a real authentication method
    if (authenticationMethod === "VercelOAuth") {
      returnTo = addQueryParam(returnTo, "vercelLogin", "true");
    }

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

function addQueryParam(url: string, key: string, value: string) {
  const symbol = url.includes("?") ? "&" : "?";
  return `${url}${symbol}${key}=${value}`;
}
