import { useRouter } from "next/router";
import type { UserProfile } from "@auth0/nextjs-auth0/client";
import { useHasOptedIn } from "api/optins";
import { withPageAuthRequired } from "@auth0/nextjs-auth0/client";
import type { InferGetServerSidePropsType, NextPage } from "next";

import { getServerSideProps } from "lib/ssr";
import { useEffect } from "react";
import { useAccessToken, useInitialData } from "hooks/useServerSideData";
import { Button } from "dashboard-common/elements/Button";
import { Callout } from "dashboard-common/elements/Callout";
import { LoadingLogo } from "dashboard-common/elements/Loading";
import Link from "next/link";

interface UserProps {
  // eslint-disable-next-line react/no-unused-prop-types
  user: UserProfile;
}

export const withAuthenticatedPage = (Page: NextPage) =>
  withPageAuthRequired(withSWRFallback(Page));

function withSWRFallback(Page: NextPage) {
  return function WithAuthenticatedPage({
    accessToken,
    initialData,
    error,
  }: InferGetServerSidePropsType<typeof getServerSideProps> & UserProps) {
    const [globalAccessToken, setAccessToken] = useAccessToken();
    const [globalInitialData, setInitialData] = useInitialData();
    useEffect(() => {
      accessToken && setAccessToken(accessToken);
    }, [accessToken, setAccessToken]);
    useEffect(() => {
      !globalInitialData && setInitialData(initialData);
    }, [initialData, setInitialData, globalInitialData]);

    if (error) {
      const message =
        error?.message || "Failed to connect to the Convex dashboard.";
      let extra = null;

      if (error?.code === "FailedToConnect") {
        extra = (
          <p>
            {" "}
            Please try again or contact us at{" "}
            <Link
              href="mailto:support@convex.dev"
              passHref
              className="items-center text-content-link dark:underline"
            >
              support@convex.dev
            </Link>{" "}
            for support with this issue.
          </p>
        );
      }

      if (error?.code === "EmailAlreadyExists") {
        extra = (
          <p>
            If this is unexepected, please contact us at{" "}
            <Link
              href="mailto:support@convex.dev"
              passHref
              className="items-center text-content-link dark:underline"
            >
              support@convex.dev
            </Link>
          </p>
        );
      }

      return (
        <div className="h-full grow">
          <div className="flex h-full flex-col items-center justify-center">
            <div className="flex w-fit flex-col gap-4">
              <Callout variant="error">
                <div className="flex flex-col gap-2">
                  {message} {extra}
                </div>
              </Callout>
              <Button
                href="/api/auth/logout"
                variant="neutral"
                className="ml-auto w-fit"
              >
                Log out
              </Button>
            </div>
          </div>
        </div>
      );
    }

    return globalAccessToken ? (
      <OptinRedirect>
        <Page />
      </OptinRedirect>
    ) : null;
  };
}

function OptinRedirect({ children }: { children: JSX.Element }) {
  const router = useRouter();
  const pathname = router.asPath !== "/" ? router.asPath : undefined;
  const { isLoading, hasOptedIn } = useHasOptedIn();
  if (!isLoading && !hasOptedIn && router.pathname !== "/accept") {
    void router.replace({
      pathname: "/accept",
      query: { from: pathname },
    });
  }

  return isLoading || (!hasOptedIn && router.pathname !== "/accept") ? (
    <div className=" flex h-full w-full items-center justify-center">
      <LoadingLogo />
    </div>
  ) : (
    children
  );
}
