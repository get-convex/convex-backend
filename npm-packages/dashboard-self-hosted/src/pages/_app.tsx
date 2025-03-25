// eslint-disable-next-line import/no-relative-packages
import "../../../dashboard-common/src/styles/globals.css";
import { AppProps } from "next/app";
import Head from "next/head";
import { useQuery } from "convex/react";
import udfs from "dashboard-common/udfs";
import { useSessionStorage } from "react-use";
import { ExitIcon, GearIcon } from "@radix-ui/react-icons";
import { ConvexLogo } from "dashboard-common/elements/ConvexLogo";
import { ToastContainer } from "dashboard-common/elements/ToastContainer";
import { ThemeConsumer } from "dashboard-common/elements/ThemeConsumer";
import { Favicon } from "dashboard-common/elements/Favicon";
import { ToggleTheme } from "dashboard-common/elements/ToggleTheme";
import { Menu, MenuItem } from "dashboard-common/elements/Menu";
import { ThemeProvider } from "next-themes";
import React, { useCallback, useEffect, useMemo, useState } from "react";
import { ErrorBoundary } from "components/ErrorBoundary";
import { DeploymentDashboardLayout } from "dashboard-common/layouts/DeploymentDashboardLayout";
import {
  DeploymentApiProvider,
  WaitForDeploymentApi,
  DeploymentInfo,
  DeploymentInfoContext,
} from "dashboard-common/lib/deploymentContext";
import { Tooltip } from "dashboard-common/elements/Tooltip";
import { DeploymentCredentialsForm } from "components/DeploymentCredentialsForm";
import { DeploymentList } from "components/DeploymentList";
import { checkDeploymentInfo } from "lib/checkDeploymentInfo";

function App({
  Component,
  pageProps: { deploymentUrl, adminKey, listDeploymentsApiUrl, ...pageProps },
}: AppProps & {
  pageProps: {
    deploymentUrl: string | null;
    adminKey: string | null;
    listDeploymentsApiUrl: string | null;
  };
}) {
  return (
    <>
      <Head>
        <title>Convex Dashboard</title>
        <meta name="description" content="Manage your Convex apps" />
        <Favicon />
      </Head>
      <ThemeProvider attribute="class" disableTransitionOnChange>
        <ThemeConsumer />
        <ToastContainer />
        <div className="flex h-screen flex-col">
          <DeploymentInfoProvider
            deploymentUrl={deploymentUrl}
            adminKey={adminKey}
            listDeploymentsApiUrl={listDeploymentsApiUrl}
          >
            <DeploymentApiProvider deploymentOverride="local">
              <WaitForDeploymentApi>
                <DeploymentDashboardLayout>
                  <Component {...pageProps} />
                </DeploymentDashboardLayout>
              </WaitForDeploymentApi>
            </DeploymentApiProvider>
          </DeploymentInfoProvider>
        </div>
      </ThemeProvider>
    </>
  );
}

const LIST_DEPLOYMENTS_API_PORT_QUERY_PARAM = "a";

function normalizeUrl(url: string) {
  try {
    const parsedUrl = new URL(url);
    // remove trailing slash
    return parsedUrl.href.replace(/\/$/, "");
  } catch (e) {
    return null;
  }
}

App.getInitialProps = async ({ ctx }: { ctx: { req?: any } }) => {
  // On server-side, get from process.env
  if (ctx.req) {
    // This is a relative URL, so add localhost as the origin so it can be parsed
    const url = new URL(ctx.req.url, "http://127.0.0.1");

    let deploymentUrl: string | null = null;
    if (process.env.NEXT_PUBLIC_DEPLOYMENT_URL) {
      deploymentUrl = normalizeUrl(process.env.NEXT_PUBLIC_DEPLOYMENT_URL);
    }

    const listDeploymentsApiPort =
      url.searchParams.get(LIST_DEPLOYMENTS_API_PORT_QUERY_PARAM) ??
      process.env.NEXT_PUBLIC_DEFAULT_LIST_DEPLOYMENTS_API_PORT;
    let listDeploymentsApiUrl: string | null = null;
    if (listDeploymentsApiPort) {
      const port = parseInt(listDeploymentsApiPort);
      if (!Number.isNaN(port)) {
        listDeploymentsApiUrl = normalizeUrl(`http://127.0.0.1:${port}`);
      }
    }

    return {
      pageProps: {
        deploymentUrl,
        adminKey: null,
        listDeploymentsApiUrl,
      },
    };
  }

  // On client-side navigation, get from window.__NEXT_DATA__
  const clientSideDeploymentUrl =
    window.__NEXT_DATA__?.props?.pageProps?.deploymentUrl ?? null;
  const clientSideAdminKey =
    window.__NEXT_DATA__?.props?.pageProps?.adminKey ?? null;
  const clientSideListDeploymentsApiUrl =
    window.__NEXT_DATA__?.props?.pageProps?.listDeploymentsApiUrl ?? null;
  return {
    pageProps: {
      deploymentUrl: clientSideDeploymentUrl ?? null,
      adminKey: clientSideAdminKey ?? null,
      listDeploymentsApiUrl: clientSideListDeploymentsApiUrl ?? null,
    },
  };
};

export default App;

const deploymentInfo: Omit<DeploymentInfo, "deploymentUrl" | "adminKey"> = {
  ok: true,
  addBreadcrumb: console.error,
  captureMessage: console.error,
  captureException: console.error,
  reportHttpError: (
    method: string,
    url: string,
    error: { code: string; message: string },
  ) => {
    console.error(
      `failed to request ${method} ${url}: ${error.code} - ${error.message} `,
    );
  },
  useCurrentTeam: () => ({
    id: 0,
    name: "Team",
    slug: "team",
  }),
  useTeamMembers: () => [],
  useTeamEntitlements: () => ({
    auditLogsEnabled: true,
  }),
  useCurrentUsageBanner: () => null,
  useCurrentProject: () => ({
    id: 0,
    name: "Project",
    slug: "project",
    teamId: 0,
  }),
  useCurrentDeployment: () => ({
    id: 0,
    name: "local",
    deploymentType: "prod",
    projectId: 0,
    kind: "local",
    previewIdentifier: null,
  }),
  useHasProjectAdminPermissions: () => true,
  useIsDeploymentPaused: () => {
    const deploymentState = useQuery(udfs.deploymentState.deploymentState);
    return deploymentState?.state === "paused";
  },
  useProjectEnvironmentVariables: () => ({ configs: [] }),
  // no-op. don't send analytics in the self-hosted dashboard.
  useLogDeploymentEvent: () => () => {},
  CloudImport: ({ sourceCloudBackupId }: { sourceCloudBackupId: number }) => (
    <div>{sourceCloudBackupId}</div>
  ),
  TeamMemberLink: () => (
    <Tooltip tip="Identity management is not available in self-hosted deployments.">
      <div className="underline decoration-dotted underline-offset-4">
        An admin
      </div>
    </Tooltip>
  ),
  ErrorBoundary: ({ children }: { children: React.ReactNode }) => (
    <ErrorBoundary>{children}</ErrorBoundary>
  ),
  useTeamUsageState: () => "Default",
  teamsURI: "",
  projectsURI: "",
  deploymentsURI: "",
  isSelfHosted: true,
  enableIndexFilters: false,
};

function DeploymentInfoProvider({
  children,
  deploymentUrl,
  adminKey,
  listDeploymentsApiUrl,
}: {
  children: React.ReactNode;
  deploymentUrl: string | null;
  adminKey: string | null;
  listDeploymentsApiUrl: string | null;
}) {
  const [shouldListDeployments, setShouldListDeployments] = useState(
    listDeploymentsApiUrl !== null,
  );
  const [isValidDeploymentInfo, setIsValidDeploymentInfo] = useState<
    boolean | null
  >(null);
  const [storedAdminKey, setStoredAdminKey] = useSessionStorage("adminKey", "");
  const [storedDeploymentUrl, setStoredDeploymentUrl] = useSessionStorage(
    "deploymentUrl",
    "",
  );
  const onSubmit = useCallback(
    async (submittedAdminKey: string, submittedDeploymentUrl: string) => {
      const isValid = await checkDeploymentInfo(
        submittedAdminKey,
        submittedDeploymentUrl,
      );
      if (isValid === false) {
        setIsValidDeploymentInfo(false);
        return;
      }
      // For deployments that don't have the `/check_admin_key` endpoint,
      // we set isValidDeploymentInfo to true so we can move on. The dashboard
      // will just hit a less graceful error later if the credentials are invalid.
      setIsValidDeploymentInfo(true);
      setStoredAdminKey(submittedAdminKey);
      setStoredDeploymentUrl(submittedDeploymentUrl);
    },
    [setStoredAdminKey, setStoredDeploymentUrl],
  );

  const finalValue: DeploymentInfo = useMemo(
    () =>
      ({
        ...deploymentInfo,
        ok: true,
        adminKey: storedAdminKey,
        deploymentUrl: storedDeploymentUrl,
      }) as DeploymentInfo,
    [storedAdminKey, storedDeploymentUrl],
  );
  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);
  useEffect(() => {
    if (typeof window !== "undefined") {
      const url = new URL(window.location.href);
      url.searchParams.delete(LIST_DEPLOYMENTS_API_PORT_QUERY_PARAM);
      window.history.replaceState({}, "", url.toString());
    }
  }, []);
  if (!mounted) return null;

  if (!isValidDeploymentInfo) {
    return (
      <div className="flex h-screen w-screen flex-col items-center justify-center gap-8">
        <ConvexLogo />
        {shouldListDeployments && listDeploymentsApiUrl !== null ? (
          <DeploymentList
            listDeploymentsApiUrl={listDeploymentsApiUrl}
            onError={() => {
              setShouldListDeployments(false);
            }}
            onSelect={onSubmit}
          />
        ) : (
          <DeploymentCredentialsForm
            onSubmit={onSubmit}
            initialAdminKey={adminKey}
            initialDeploymentUrl={deploymentUrl}
          />
        )}
        {isValidDeploymentInfo === false && (
          <div className="text-sm text-content-secondary">
            The deployment URL or admin key is invalid. Please check that you
            have entered the correct values.
          </div>
        )}
      </div>
    );
  }
  return (
    <>
      <Header
        onLogout={() => {
          setStoredAdminKey("");
          setStoredDeploymentUrl("");
        }}
      />
      <DeploymentInfoContext.Provider value={finalValue}>
        <ErrorBoundary>{children}</ErrorBoundary>
      </DeploymentInfoContext.Provider>
    </>
  );
}

function Header({ onLogout }: { onLogout: () => void }) {
  if (process.env.NEXT_PUBLIC_HIDE_HEADER) {
    return null;
  }

  return (
    <header className="-ml-1 flex min-h-[56px] items-center justify-between gap-1 overflow-x-auto border-b bg-background-secondary pr-4 scrollbar-none sm:gap-6">
      <ConvexLogo height={64} width={192} />
      <Menu
        buttonProps={{
          icon: (
            <GearIcon className="h-7 w-7 rounded p-1 text-content-primary hover:bg-background-tertiary" />
          ),
          variant: "unstyled",
          "aria-label": "Dashboard Settings",
        }}
        placement="bottom-end"
      >
        <ToggleTheme />
        <MenuItem action={onLogout}>
          <div className="flex w-full items-center justify-between">
            Log Out
            <ExitIcon className="text-content-secondary" />
          </div>
        </MenuItem>
      </Menu>
    </header>
  );
}
