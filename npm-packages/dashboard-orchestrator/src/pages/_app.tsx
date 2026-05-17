// eslint-disable-next-line import/no-relative-packages
import "../../../@convex-dev/design-system/src/styles/shared.css";
// eslint-disable-next-line import/no-relative-packages
import "../../../dashboard-common/src/styles/globals.css";
import { AppProps } from "next/app";
import Head from "next/head";
import { useRouter } from "next/router";
import { SWRConfig } from "swr";
import { ToastContainer } from "@common/elements/ToastContainer";
import { ThemeConsumer } from "@common/elements/ThemeConsumer";
import { Favicon } from "@common/elements/Favicon";
import { ThemeProvider } from "next-themes";
import Link from "next/link";
import { UIProvider } from "@ui/UIContext";
import { OrchestratorDeploymentShell } from "../components/OrchestratorDeploymentShell";
import { OrchestratorHeader } from "../components/OrchestratorHeader";
import { ErrorBoundary } from "../components/ErrorBoundary";

function isDeploymentRoute(pathname: string): boolean {
  return /^\/t\/\[team\]\/\[project\]\/\[deploymentName\](\/|$)/.test(pathname);
}

function isAuthRoute(pathname: string): boolean {
  // Standalone (no orchestrator header) auth flows: sign-in, password reset,
  // email verification, and invite acceptance (which redirects to /login if
  // the user isn't signed in).
  return (
    pathname === "/login" ||
    pathname === "/forgot-password" ||
    pathname === "/reset-password" ||
    pathname === "/verify-email" ||
    pathname === "/invite/[code]"
  );
}

export default function App({ Component, pageProps }: AppProps) {
  const router = useRouter();
  const isDeployment = isDeploymentRoute(router.pathname);
  const isAuth = isAuthRoute(router.pathname);

  return (
    <>
      <Head>
        <title>Convex Orchestrator</title>
        <meta
          name="description"
          content="Manage your self-hosted Convex deployments"
        />
        <Favicon />
      </Head>
      <UIProvider Link={Link}>
        <SWRConfig
          value={{
            // Don't refetch on focus/reconnect — orchestrator data (teams,
            // projects, deployments, auth) doesn't change every time you
            // alt-tab. Avoids waterfalls of round-trips on tab switches.
            revalidateOnFocus: false,
            revalidateOnReconnect: false,
            // Keep cached data fresh for 30s before considering it stale
            // enough to refetch on a remount. Big win for navigation between
            // deployment routes — the shell reads cached teams/projects/auth
            // instead of hitting the orchestrator each time.
            dedupingInterval: 30_000,
            // Render with previous data while a refetch is in flight, so
            // navigations don't show a Loading state when the data is
            // already available.
            keepPreviousData: true,
            shouldRetryOnError: false,
          }}
        >
          <ThemeProvider attribute="class" disableTransitionOnChange>
            <ThemeConsumer />
            <ToastContainer />
            <div className="flex h-screen flex-col">
              <ErrorBoundary>
                {!isAuth && <OrchestratorHeader />}
                <div className="flex min-h-0 flex-1 flex-col">
                  {isDeployment ? (
                    <OrchestratorDeploymentShell>
                      <Component {...pageProps} />
                    </OrchestratorDeploymentShell>
                  ) : (
                    <Component {...pageProps} />
                  )}
                </div>
              </ErrorBoundary>
            </div>
          </ThemeProvider>
        </SWRConfig>
      </UIProvider>
    </>
  );
}
