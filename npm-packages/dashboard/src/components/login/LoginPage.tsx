import classNames from "classnames";
import { useState } from "react";
import { Spinner } from "@ui/Spinner";
import { buttonClasses } from "@ui/Button";

import GithubLogo from "logos/github-logo.svg";

export function LoginPage({ returnTo }: { returnTo?: string }) {
  const [isLoggingIn, setIsLoggingIn] = useState(false);
  return (
    <div className="flex flex-col items-center">
      <a
        onClick={() => setIsLoggingIn(true)}
        href={`/api/auth/login${returnTo ? `?returnTo=${returnTo}` : ""}`}
        className={classNames(
          "mb-4",
          buttonClasses({
            variant: "primary",
            size: "md",
            // We're getting the disabled styles for the button, but not actually disabled the anchor
            // Because this isLoggingIn bool is just a heuristic, we don't want to actually disable the button
            // in case the user wants to click again.
            disabled: isLoggingIn,
          }),
        )}
      >
        {isLoggingIn ? (
          <>
            <Spinner className="h-6 w-6" /> Logging in...
          </>
        ) : (
          <>
            <GithubLogo className="fill-white" /> Log in with GitHub
          </>
        )}
      </a>
    </div>
  );
}
