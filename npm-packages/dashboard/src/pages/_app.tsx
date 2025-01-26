// eslint-disable-next-line import/no-relative-packages
import "../../../dashboard-common/src/styles/globals.css";
import type { AppProps } from "next/app";
import { useAuth0 } from "hooks/useAuth0";
import { useRouter } from "next/router";
import React, { useEffect } from "react";
import { setUser, ErrorBoundary } from "@sentry/nextjs";
import {
  DeploymentInfoProvider,
  MaybeDeploymentApiProvider,
} from "hooks/deploymentApi";
import { SWRConfig } from "swr";
import { swrConfig } from "hooks/swrConfig";
import {
  asyncWithLDProvider,
  withLDConsumer,
} from "launchdarkly-react-client-sdk";
import { flagDefaultsKebabCase } from "hooks/useLaunchDarkly";
import { DashboardLayout } from "layouts/DashboardLayout";
import { DashboardHeader } from "components/header/DashboardHeader";
import { UserProvider, useUser } from "@auth0/nextjs-auth0/client";
import { useAsync } from "react-use";
import { basicLogger, LDClient, LDFlagSet } from "launchdarkly-js-client-sdk";
import { useAccessToken, useInitialData } from "hooks/useServerSideData";
import { useRouterProgress } from "hooks/useRouterProgress";
import Head from "next/head";
import { RefreshSession } from "components/login/RefreshSession";
import { useDashboardVersion } from "hooks/api";
import { useProfile } from "api/profile";
import { useGlobalLDContext, useLDContext } from "hooks/useLaunchDarklyContext";
import {
  Favicon,
  LoadingLogo,
  ThemeConsumer,
  ToastContainer,
  ThemeProvider,
} from "dashboard-common";
import { CurrentDeploymentDashboardLayout } from "layouts/DeploymentDashboardLayout";
import { Fallback } from "./500";

// LaunchDarkly cleaned up their API and not longer exposes this type
/**
 * The possible props the wrapped component can receive from the `LDConsumer` HOC.
 */
export interface LDProps {
  /**
   * A map of feature flags from their keys to their values.
   * Keys are camelCased using `lodash.camelcase`.
   */
  flags?: LDFlagSet;

  /**
   * An instance of `LDClient` from the LaunchDarkly JS SDK (`launchdarkly-js-client-sdk`)
   *
   * @see https://docs.launchdarkly.com/sdk/client-side/javascript
   */
  ldClient?: LDClient;
}

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
      <ThemeProvider attribute="class" disableTransitionOnChange>
        <ThemeConsumer />
        <UserProvider user={pageProps.user}>
          <RefreshSession />
          <SentryUserProvider>
            <ErrorBoundary fallback={Fallback}>
              <SWRConfig value={{ ...swrConfig(), fallback: { initialData } }}>
                <ToastContainer />

                {inUnauthedRoute ? (
                  <Component {...pageProps} />
                ) : (
                  <MaybeLaunchDarklyProvider>
                    <LaunchDarklyConsumer>
                      <div className="flex h-screen flex-col">
                        <DashboardHeader />
                        {inDeployment ? (
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
                    </LaunchDarklyConsumer>
                  </MaybeLaunchDarklyProvider>
                )}
              </SWRConfig>
            </ErrorBoundary>
          </SentryUserProvider>
        </UserProvider>
      </ThemeProvider>
    </>
  );
}

function SentryUserProvider({ children }: { children: React.ReactElement }) {
  const { user } = useAuth0();
  const profile = useProfile();
  useEffect(() => {
    if (user) {
      setUser({
        id: user.sub ?? undefined,
        email: (profile?.email || user.email) ?? undefined,
      });
    }
  }, [profile?.email, user]);

  return children;
}

function LaunchDarklyProvider({ children }: { children: React.ReactNode }) {
  const { user } = useUser();
  const router = useRouter();

  const clientSideID = process.env.NEXT_PUBLIC_LAUNCHDARKLY_SDK_CLIENT_SIDE_ID;
  if (!clientSideID) {
    throw new Error("LaunchDarkly Client Side ID not set");
  }

  const [, setContext] = useGlobalLDContext();
  const localContext = useLDContext(user);
  useEffect(() => {
    !router.query.deploymentName && localContext && setContext(localContext);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [JSON.stringify(localContext), router.query.deploymentName]);

  // For some reason, typing this properly still causes a type error ts(7022)
  const { value: LDProvider }: any = useAsync<
    () => ReturnType<typeof asyncWithLDProvider>
  >(
    async () =>
      LDProvider ||
      asyncWithLDProvider({
        clientSideID,
        options: {
          fetchGoals: false,
          logger: basicLogger({ level: "error" }),
        },
        flags: flagDefaultsKebabCase,
        // If in test mode, default the user to a test user with a stable key.
        context: process.env.NEXT_PUBLIC_TEST_MODE
          ? { key: "test", anonymous: true, kind: "user" }
          : // Otherwise, use an anonymous user with a stable key until we are logged in.
            // This prevents too many users from being created in LaunchDarkly.
            { key: "user", anonymous: true, kind: "user" },
      }),
    [],
  );

  return LDProvider ? (
    <LDProvider>{children}</LDProvider>
  ) : (
    <div className="flex h-screen w-full items-center justify-center">
      <LoadingLogo />
    </div>
  );
}

function MaybeLaunchDarklyProvider({
  children,
}: {
  children: React.ReactNode;
}) {
  const [accessToken] = useAccessToken();

  return accessToken ? (
    <LaunchDarklyProvider>{children}</LaunchDarklyProvider>
  ) : (
    // eslint-disable-next-line react/jsx-no-useless-fragment
    <>{children}</>
  );
}

const LaunchDarklyConsumer = withLDConsumer({ clientOnly: true })(
  function LaunchDarklyConsumer({
    ldClient,
    children,
  }: LDProps & { children: React.ReactElement }) {
    const [context] = useGlobalLDContext();
    useEffect(() => {
      // Don't identify users in test mode
      if (process.env.NEXT_PUBLIC_TEST_MODE) {
        return;
      }
      context && void ldClient?.identify(context);
      // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [JSON.stringify(context)]);
    return children;
  },
);
