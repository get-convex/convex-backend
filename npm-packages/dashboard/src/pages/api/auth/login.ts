import { NextApiRequest, NextApiResponse } from "next";
import { WorkOS } from "@workos-inc/node";

export default function handler(req: NextApiRequest, res: NextApiResponse) {
  if (req.method !== "GET") {
    return res.status(405).json({ error: "Method not allowed" });
  }

  const workos = new WorkOS(process.env.WORKOS_API_SECRET, {
    clientId: process.env.WORKOS_CLIENT_ID,
  });

  try {
    // Get the returnTo parameter from the query string
    const returnTo = (req.query.returnTo as string) || "/";
    const provider = "authkit";

    const authorizationUrl = workos.userManagement.getAuthorizationUrl({
      // Specify that we'd like AuthKit to handle the authentication flow
      // Can be GitHubOAuth, GoogleOAuth, etc.
      provider,
      // The callback endpoint that WorkOS will redirect to after a user authenticates
      redirectUri: `${
        process.env.WORKOS_REDIRECT_URI ||
        `https://${process.env.WORKOS_REDIRECT_URI_OVERRIDE}` ||
        ""
      }/api/auth/callback`,
      clientId: process.env.WORKOS_CLIENT_ID || "",
      // Pass the returnTo URL as state so it can be used in the callback
      state: returnTo,
    });

    // Redirect the user to the AuthKit sign-in page
    res.redirect(authorizationUrl);
  } catch (error) {
    console.error("Error generating authorization URL:", error);
    res.status(500).json({ error: "Internal server error" });
  }
}
