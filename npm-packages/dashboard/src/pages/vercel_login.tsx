import { useSessionStorage } from "react-use";
import { useRouter } from "next/router";
import { useEffect } from "react";
import { LoadingLogo } from "@ui/Loading";

/**
 * This page handles the Vercel OAuth login flow integration with Convex's authentication system.
 * It acts as a bridge between Vercel's OAuth initiation and Auth0, storing the Vercel code temporarily
 * in session storage and redirecting appropriately based on the presence of state and resume parameters.
 * This enables users to authenticate with their Vercel account credentials.
 */
export default function VercelLogin() {
  const [vercelCode, setVercelCode] = useSessionStorage<string | undefined>(
    "vercelCode",
  );
  const { replace, query, isReady } = useRouter();
  useEffect(() => {
    if (!isReady) {
      return;
    }
    if (!query.resume) {
      // Resume parameter is not present, so we start the login flow.
      if (!query.code) {
        // There's no code query parameter, so this request was likely not initiated by Vercel.
        // We redirect to the regular login page.
        void replace("/login");
        return;
      }
      // We have a code query parameter, so we store it
      // in session storage (so it's accessible once we're redirected back and redirect to the Auth0 login page.
      setVercelCode(query.code.toString());
      void replace("/api/auth/login?connection=vercel");
    }

    // If both state and resume parameters are present,
    // we were likely redirected back from Big Brain's /vercel/authorize endpoint.
    // If state is invalid, we won't be able to proceed.
    // Check to make sure a vercel code was previusly stored, and if so,
    // continue the login process with Auth0.
    if (query.state && query.resume) {
      if (!vercelCode) {
        return;
      }
      const code = vercelCode;
      setVercelCode(undefined);
      void replace(
        `${process.env.NEXT_PUBLIC_AUTH0_ISSUER_BASE_URL}/login/callback?code=${code}&state=${query.state}`,
      );
    }
  }, [isReady, query, replace, setVercelCode, vercelCode]);
  return (
    <div className="flex h-screen w-screen items-center justify-center">
      <LoadingLogo />
    </div>
  );
}
