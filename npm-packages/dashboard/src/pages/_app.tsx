// eslint-disable-next-line import/no-relative-packages
import "../../../dashboard-common/src/styles/globals.css";
import type { AppProps } from "next/app";
import { useRouter } from "next/router";
import React from "react";
import { ErrorBoundary } from "@sentry/nextjs";
import { SWRConfig } from "swr";
import { swrConfig } from "hooks/swrConfig";
import { DashboardLayout } from "layouts/DashboardLayout";
import { DashboardHeader } from "components/header/DashboardHeader";
import { UserProvider } from "@auth0/nextjs-auth0/client";
import { useInitialData } from "hooks/useServerSideData";
import { useRouterProgress } from "hooks/useRouterProgress";
import Head from "next/head";
import { RefreshSession } from "components/login/RefreshSession";
import { useDashboardVersion } from "hooks/api";
import { Favicon } from "dashboard-common/elements/Favicon";
import { ThemeConsumer } from "dashboard-common/elements/ThemeConsumer";
import { ToastContainer } from "dashboard-common/elements/ToastContainer";
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
import { Fallback } from "./500";

declare global {
  interface Window {
    gtag: any;
  }
}

if (typeof window !== "undefined") {
  // tells analytics that this is not frontend
  (window as any).isConsole = true;
}

const UNAUTHED_ROUTES = ["/404", "/login", "/signup"];

export default function App({ Component, pageProps }: AppProps) {
  const router = useRouter();
  const pathWithoutQueryString = router.asPath.split("?")[0].split("#")[0];

  const inUnauthedRoute = UNAUTHED_ROUTES.some((r) => r === router.pathname);
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
      <PostHogProvider>
        <ThemeProvider attribute="class" disableTransitionOnChange>
          <ThemeConsumer />
          <UserProvider user={pageProps.user}>
            <RefreshSession />
            <SentryUserProvider>
              <ErrorBoundary fallback={Fallback}>
                <SWRConfig
                  value={{ ...swrConfig(), fallback: { initialData } }}
                >
                  <ToastContainer />

                  {inUnauthedRoute ? (
                    <Component {...pageProps} />
                  ) : (
                    <MaybeLaunchDarklyProvider>
                      <LaunchDarklyConsumer>
                        <div className="flex h-screen flex-col overflow-y-hidden">
                          <DashboardHeader />
                          {inDeployment ? (
                            <div className="relative flex h-full flex-col">
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
                            </div>
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
                      </LaunchDarklyConsumer>
                    </MaybeLaunchDarklyProvider>
                  )}
                </SWRConfig>
              </ErrorBoundary>
            </SentryUserProvider>
          </UserProvider>
        </ThemeProvider>
      </PostHogProvider>
    </>
  );
}
