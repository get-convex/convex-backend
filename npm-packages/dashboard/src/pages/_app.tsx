import "../styles/global.css";

import type { AppProps } from "next/app";
import { useRouter } from "next/router";
import React from "react";
import { ErrorBoundary } from "@sentry/nextjs";
import { SWRConfig } from "swr";
import { swrConfig } from "hooks/swrConfig";
import { DashboardLayout } from "layouts/DashboardLayout";
import { DashboardHeader } from "components/header/DashboardHeader";
import { useInitialData } from "hooks/useServerSideData";
import { useRouterProgress } from "hooks/useRouterProgress";
import Head from "next/head";

import { useDashboardVersion } from "hooks/useDashboardVersion";
import { Favicon } from "@common/elements/Favicon";
import { ThemeConsumer } from "@common/elements/ThemeConsumer";
import { ToastContainer } from "@common/elements/ToastContainer";
import { ThemeProvider } from "next-themes";
import { CurrentDeploymentDashboardLayout } from "layouts/DeploymentDashboardLayout";
import { DeploymentInfoProvider } from "providers/DeploymentInfoProvider";
import { MaybeDeploymentApiProvider } from "providers/MaybeDeploymentApiProvider";
import { PostHogProvider } from "providers/PostHogProvider";
import { SentryUserProvider } from "providers/SentryUserProvider";
import {
  LaunchDarklyConsumer,
  MaybeLaunchDarklyProvider,
} from "providers/LaunchDarklyProviders";
import { CommandPalette } from "elements/CommandPalette";
import { Fallback } from "pages/500";
import { UIProvider } from "@ui/UIContext";
import Link from "next/link";
import { RefreshSession } from "components/login/RefreshSession";
import { AuthProvider } from "providers/AuthProvider";
import { useSSOLoginRequired } from "api/api";
import { Sheet } from "@ui/Sheet";
import { Button } from "@ui/Button";
import { ExitIcon, LockClosedIcon } from "@radix-ui/react-icons";

declare global {
  interface Window {
    gtag: any;
  }
}

if (typeof window !== "undefined") {
  // tells analytics that this is not frontend
  (window as any).isConsole = true;
}

const UNAUTHED_ROUTES = [
  "/404",
  "/login",
  "/signup",
  /^\/referral\/[A-Z0-9]+$/,
];

export default function App({ Component, pageProps }: AppProps) {
  const [ssoLoginRequired] = useSSOLoginRequired();
  const router = useRouter();
  const pathWithoutQueryString = router.asPath.split("?")[0].split("#")[0];

  const inUnauthedRoute = UNAUTHED_ROUTES.some((r) =>
    typeof r === "string" ? r === router.pathname : r.test(router.pathname),
  );
  // To share state across page transitions we load deployment data in this
  // shared App component if the path looks like a deployment.
  const inDeployment = router.pathname.startsWith(
    "/t/[team]/[project]/[deploymentName]",
  );

  const [initialData] = useInitialData();

  useRouterProgress();

  useDashboardVersion();

  return (
    <>
      <Head>
        <title>Convex Dashboard</title>
        <meta name="description" content="Manage your Convex apps" />
        <Favicon />
      </Head>
      <UIProvider Link={Link}>
        <AuthProvider>
          <PostHogProvider>
            <ThemeProvider attribute="class" disableTransitionOnChange>
              <ThemeConsumer />
              <MaybeLaunchDarklyProvider>
                <LaunchDarklyConsumer>
                  <>
                    <RefreshSession />
                    <SentryUserProvider>
                      <ErrorBoundary fallback={Fallback}>
                        <SWRConfig
                          value={{
                            ...swrConfig(),
                            fallback: { initialData },
                          }}
                        >
                          <ToastContainer />

                          {inUnauthedRoute ? (
                            <Component {...pageProps} />
                          ) : (
                            <div className="flex h-screen flex-col">
                              <CommandPalette />
                              <DashboardHeader />
                              {!!ssoLoginRequired &&
                              ssoLoginRequired === router.query.team ? (
                                <div className="flex h-full w-full items-center justify-center">
                                  <Sheet className="flex max-w-prose flex-col gap-4">
                                    <div className="flex items-center gap-2">
                                      <LockClosedIcon className="size-8" />
                                      <h3>Single Sign-On Login Required</h3>
                                    </div>
                                    <span className="flex flex-col gap-2">
                                      <p>
                                        This team requires you to log in with
                                        Single Sign-On to access it.
                                      </p>
                                      <p>
                                        You may log out and log back in through
                                        your Single Sign-On provider, or switch
                                        teams by using the selector on the top
                                        of this page.
                                      </p>
                                    </span>
                                    <Button
                                      className="ml-auto w-fit"
                                      href="/api/auth/logout"
                                      icon={<ExitIcon />}
                                    >
                                      Log Out
                                    </Button>
                                  </Sheet>
                                </div>
                              ) : inDeployment ? (
                                <DeploymentInfoProvider>
                                  <MaybeDeploymentApiProvider>
                                    <CurrentDeploymentDashboardLayout>
                                      <ErrorBoundary
                                        fallback={Fallback}
                                        key={pathWithoutQueryString}
                                      >
                                        <Component {...pageProps} />
                                      </ErrorBoundary>
                                    </CurrentDeploymentDashboardLayout>
                                  </MaybeDeploymentApiProvider>
                                </DeploymentInfoProvider>
                              ) : (
                                <DashboardLayout>
                                  <ErrorBoundary
                                    fallback={Fallback}
                                    key={pathWithoutQueryString}
                                  >
                                    <Component {...pageProps} />
                                  </ErrorBoundary>
                                </DashboardLayout>
                              )}
                            </div>
                          )}
                        </SWRConfig>
                      </ErrorBoundary>
                    </SentryUserProvider>
                  </>
                </LaunchDarklyConsumer>
              </MaybeLaunchDarklyProvider>
            </ThemeProvider>
          </PostHogProvider>
        </AuthProvider>
      </UIProvider>
    </>
  );
}
