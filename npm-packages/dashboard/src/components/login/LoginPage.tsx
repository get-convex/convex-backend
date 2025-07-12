import { useState } from "react";
import { Button } from "@ui/Button";

import GithubLogo from "logos/github-logo.svg";
import GoogleLogo from "logos/google.svg";
import { Sheet } from "@ui/Sheet";
import { UIProvider } from "@ui/UIContext";
import { LoadingLogo } from "@ui/Loading";

export function LoginPage({ returnTo }: { returnTo?: string }) {
  const [isLoggingIn, setIsLoggingIn] = useState(false);
  return (
    <div className="flex flex-col items-center">
      <Sheet className="flex flex-col items-center">
        <h2 className="mb-4">Welcome to Convex</h2>
        <p className="mb-6">Log in to your account to continue.</p>
        {isLoggingIn ? (
          <LoadingLogo />
        ) : (
          <UIProvider>
            <div className="flex w-full flex-col items-center gap-4">
              <Button
                onClickOfAnchorLink={() => {
                  setIsLoggingIn(true);
                }}
                href={`/api/auth/login?connection=github${returnTo ? `&returnTo=${returnTo}` : ""}`}
                icon={<GithubLogo className="mr-2 dark:fill-white" />}
                variant="neutral"
                size="md"
                className="w-fit"
              >
                Continue with GitHub
              </Button>
              <Button
                onClickOfAnchorLink={() => {
                  setIsLoggingIn(true);
                }}
                href={`/api/auth/login?connection=google-oauth2${returnTo ? `&returnTo=${returnTo}` : ""}`}
                variant="neutral"
                icon={<GoogleLogo className="mr-2" />}
                className="w-fit"
                size="md"
              >
                Continue with Google
              </Button>
            </div>
          </UIProvider>
        )}
        <p className="mt-6 max-w-prose text-xs text-pretty text-content-secondary">
          Don't have an account? Clicking one of the buttons above will create
          one for you.
        </p>
      </Sheet>
    </div>
  );
}
