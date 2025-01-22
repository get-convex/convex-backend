import { NextApiRequest, NextApiResponse } from "next";
import { auth0 } from "server/auth0";

const authorizationParams = {
  // This value was created before we renamed console to dashboard, and cannot be changed easily.
  // It's just a fixed string used for identifying the Auth0 token, so it's fine and not user-facing.
  audience: "https://console.convex.dev/api/",
  scope: "openid profile email list:instances manage:instances offline_access",
};

export default auth0().handleAuth({
  login: auth0().handleLogin((req) => {
    if (!("query" in req)) throw new Error("Why is this a NextRequest?");
    const returnTo =
      req.query.returnTo && !req.query.returnTo.toString().startsWith("/api")
        ? req.query.returnTo.toString()
        : "";
    const connection = req.query.useEmail
      ? "Username-Password-Authentication"
      : "github";

    return {
      returnTo,
      authorizationParams: {
        ...authorizationParams,
        connection,
      },
    };
  }),

  refresh: async (req: NextApiRequest, res: NextApiResponse) => {
    if (req.method !== "POST") {
      res.setHeader("Allow", "POST");
      res.status(405).end("Method Not Allowed");
      return;
    }
    try {
      const { accessToken } = await auth0().getAccessToken(req, res);
      res.status(200).json({ accessToken });
    } catch (error: any) {
      res.status(500).json({ error: error.message });
    }
  },

  logout: async (req: NextApiRequest, res: NextApiResponse) => {
    const token = await auth0().getAccessToken(req, res);
    res.setHeader("Set-Cookie", [`appSession=deleted; Max-Age=-1; Path=/;`]);
    res.redirect(
      `${process.env.AUTH0_ISSUER_BASE_URL}/oidc/logout?${new URLSearchParams({
        client_id: process.env.AUTH0_CLIENT_ID as string,
        logout_hint: token.accessToken || "",
        post_logout_redirect_uri: `${
          // Use the internal env variable, or the NEXT_PUBLIC variable if it's not set (preview environments).
          process.env.AUTH0_BASE_URL ||
          // We should only get here on preview deployments, and this value should be set to the VERCEL_URL value,
          // which doesn't have the https:// prefix.
          `https://${process.env.NEXT_PUBLIC_AUTH0_BASE_URL}`
        }/login`,
      })}`,
    );
  },
});
