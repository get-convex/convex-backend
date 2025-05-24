import { LoginLayout } from "layouts/LoginLayout";
import { Sheet } from "@ui/Sheet";
import { Button } from "@ui/Button";
import GithubLogo from "logos/github-logo.svg";
import GoogleLogo from "logos/google.svg";
import { Spinner } from "@ui/Spinner";
import React from "react";
import { useRouter } from "next/router";
import { useAuth0 } from "hooks/useAuth0";
import {
  LinkIdentityState,
  providerToDisplayName,
} from "components/profile/ConnectedIdentities";

export function LinkIdentityForm({
  resume,
  status,
  message,
  accessToken,
  setLinkIdentityState,
  providerDisplayName,
  provider,
}: {
  resume: string | undefined;
  status: "waitingForCookie" | "ready" | "pending" | "error";
  message: string;
  accessToken: string;
  setLinkIdentityState: (v: LinkIdentityState) => void;
  providerDisplayName: string;
  provider: string;
}) {
  const { user } = useAuth0();
  const router = useRouter();
  const providerHint =
    router.query.hint === "undefined"
      ? undefined
      : (router.query.hint as string | undefined);
  return (
    <div className="h-screen">
      <LoginLayout>
        <Sheet>
          <h2 className="mb-4">Link Existing Account</h2>
          {resume && (status === "waitingForCookie" || status === "ready") && (
            <p className="flex max-w-prose flex-col gap-4 text-pretty">
              Invalid account link state.
              <Button href="/" className="w-fit">
                Proceed to the dashboard
              </Button>
            </p>
          )}
          {status === "pending" && (
            <div className="flex h-[5.875rem] w-full items-center gap-1">
              <Spinner className="ml-0" />
              <p>Linking your account</p>
            </div>
          )}
          {status === "error" && (
            <div className="flex flex-col gap-4">
              <p className="max-w-prose text-pretty text-content-error">
                {message}
              </p>
              <div className="flex gap-2">
                {accessToken && (
                  <Button href="/profile" className="w-fit" variant="neutral">
                    Go to profile
                  </Button>
                )}
                <Button
                  variant="neutral"
                  onClick={() => {
                    setLinkIdentityState({});
                    void router.push("/api/auth/logout");
                  }}
                  className="w-fit"
                >
                  Log Out
                </Button>
              </div>
            </div>
          )}
          {!resume && (
            <>
              <p className="mb-3 max-w-prose text-pretty">
                To use your{" "}
                <span className="font-semibold">
                  {providerDisplayName} account (
                  {provider === "google-oauth2"
                    ? user?.email
                    : provider === "github"
                      ? user?.nickname
                      : user?.sub?.split("|"[1])}
                  )
                </span>{" "}
                with Convex, you must link it to your existing Convex account.
              </p>
              <p className="mb-6 max-w-prose text-pretty">
                {providerHint
                  ? `Log in with ${providerToDisplayName[providerHint]} to continue:`
                  : "Select the authentication method of your existing Convex authentication method to continue:"}
              </p>
              <div className="flex flex-wrap gap-3">
                {(!providerHint || providerHint === "github") && (
                  <Button
                    href="/api/auth/login?connection=github&returnTo=/link_identity?resume=true"
                    icon={<GithubLogo className="mr-2 dark:fill-white" />}
                    size="md"
                    variant="neutral"
                    className="w-fit"
                    disabled={
                      provider === "github" || status === "waitingForCookie"
                    }
                    tip={
                      provider === "github"
                        ? "You cannot link multiple GitHub accounts to Convex. Please contact support to merge your accounts."
                        : undefined
                    }
                    loading={status === "waitingForCookie"}
                  >
                    Continue with GitHub
                  </Button>
                )}
                {(!providerHint || providerHint === "google-oauth2") && (
                  <Button
                    href="/api/auth/login?connection=google-oauth2&returnTo=/link_identity?resume=true"
                    icon={<GoogleLogo className="mr-2 dark:fill-white" />}
                    size="md"
                    variant="neutral"
                    className="w-fit"
                    loading={status === "waitingForCookie"}
                    disabled={status === "waitingForCookie"}
                  >
                    Continue with Google
                  </Button>
                )}
              </div>
            </>
          )}
          <p className="mt-6 text-pretty text-xs text-content-secondary">
            Having trouble logging in? Contact us at{" "}
            <a
              className="text-content-link hover:underline"
              href="mailto:support@convex.dev"
            >
              support@convex.dev
            </a>{" "}
            to get help accessing your account.
          </p>
        </Sheet>
      </LoginLayout>
    </div>
  );
}
