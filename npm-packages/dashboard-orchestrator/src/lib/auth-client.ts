// Client-side BetterAuth handle. Uses the same origin as the dashboard app,
// so cookies attach automatically on same-origin requests.

import { createAuthClient } from "better-auth/react";

export const authClient = createAuthClient({
  baseURL:
    process.env.NEXT_PUBLIC_BETTER_AUTH_URL ??
    (typeof window !== "undefined" ? window.location.origin : ""),
});

export const {
  useSession,
  signIn,
  signUp,
  signOut,
  forgetPassword,
  resetPassword,
  sendVerificationEmail,
} = authClient;
