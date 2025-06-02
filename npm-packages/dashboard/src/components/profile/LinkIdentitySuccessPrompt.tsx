import { LoginLayout } from "layouts/LoginLayout";
import { Sheet } from "@ui/Sheet";
import { Button } from "@ui/Button";
import GithubLogo from "logos/github-logo.svg";
import GoogleLogo from "logos/google.svg";
import React from "react";
import { UIProvider } from "@ui/UIContext";

export function LinkIdentitySuccessPrompt() {
  return (
    <div className="h-screen">
      <LoginLayout>
        <Sheet>
          <h2 className="mb-4">Log in to continue</h2>
          <p className="mb-6 max-w-prose text-pretty">
            Your accounts have been linked. Please log in again to continue:
          </p>
          <UIProvider>
            <div className="flex flex-wrap gap-3">
              <Button
                href="/api/auth/login?connection=github&returnTo=/profile"
                icon={<GithubLogo className="mr-2 dark:fill-white" />}
                size="md"
                variant="neutral"
                className="w-fit"
              >
                Continue with GitHub
              </Button>
              <Button
                href="/api/auth/login?connection=google-oauth2&returnTo=/profile"
                icon={<GoogleLogo className="mr-2 dark:fill-white" />}
                size="md"
                variant="neutral"
                className="w-fit"
              >
                Continue with Google
              </Button>
            </div>
          </UIProvider>
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
