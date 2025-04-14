// eslint-disable-next-line import/no-relative-packages
import "../../../dashboard-common/src/styles/globals.css";
// eslint-disable-next-line import/no-relative-packages
import "../../../ui/src/styles/shared.css";
import { AppProps } from "next/app";
import Head from "next/head";
import { useQuery } from "convex/react";
import udfs from "@common/udfs";
import { useSessionStorage } from "react-use";
import { ExitIcon, GearIcon } from "@radix-ui/react-icons";
import { ConvexLogo } from "@common/elements/ConvexLogo";
import { ToastContainer } from "@common/elements/ToastContainer";
import { ThemeConsumer } from "@common/elements/ThemeConsumer";
import { Favicon } from "@common/elements/Favicon";
import { ToggleTheme } from "@common/elements/ToggleTheme";
import { Menu, MenuItem } from "@ui/Menu";
import { ThemeProvider } from "next-themes";
import React, { useCallback, useEffect, useMemo, useState } from "react";
import { ErrorBoundary } from "components/ErrorBoundary";
import { DeploymentDashboardLayout } from "@common/layouts/DeploymentDashboardLayout";
import {
  DeploymentApiProvider,
  WaitForDeploymentApi,
  DeploymentInfo,
  DeploymentInfoContext,
} from "@common/lib/deploymentContext";
import { Tooltip } from "@ui/Tooltip";
import { DeploymentCredentialsForm } from "components/DeploymentCredentialsForm";
import { DeploymentList } from "components/DeploymentList";
import { checkDeploymentInfo } from "lib/checkDeploymentInfo";
import { ConvexCloudReminderToast } from "components/ConvexCloudReminderToast";
import { z } from "zod";

function App({
  Component,
  pageProps: {
    deploymentUrl,
    adminKey,
    defaultListDeploymentsApiUrl,
    ...pageProps
  },
}: AppProps & {
  pageProps: {
    deploymentUrl: string | null;
    adminKey: string | null;
    defaultListDeploymentsApiUrl: string | null;
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
            defaultListDeploymentsApiUrl={defaultListDeploymentsApiUrl}
          >
            <DeploymentApiProvider deploymentOverride="local">
              <WaitForDeploymentApi>
                <DeploymentDashboardLayout>
                  <>
                    <Component {...pageProps} />
                    <ConvexCloudReminderToast />
                  </>
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
const SELECTED_DEPLOYMENT_NAME_QUERY_PARAM = "d";
const SESSION_STORAGE_DEPLOYMENT_NAME_KEY = "deploymentName";

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
    // Note -- we can't use `ctx.req.url` when serving the dashboard statically,
    // so instead we'll read from query params on the client side.

    let deploymentUrl: string | null = null;
    if (process.env.NEXT_PUBLIC_DEPLOYMENT_URL) {
      deploymentUrl = normalizeUrl(process.env.NEXT_PUBLIC_DEPLOYMENT_URL);
    }

    const listDeploymentsApiPort =
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
        defaultListDeploymentsApiUrl: listDeploymentsApiUrl,
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
  const clientSideSelectedDeploymentName =
    window.__NEXT_DATA__?.props?.pageProps?.selectedDeploymentName ?? null;
  return {
    pageProps: {
      deploymentUrl: clientSideDeploymentUrl ?? null,
      adminKey: clientSideAdminKey ?? null,
      defaultListDeploymentsApiUrl: clientSideListDeploymentsApiUrl ?? null,
      selectedDeploymentName: clientSideSelectedDeploymentName ?? null,
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
  useCurrentDeployment: () => {
    const [storedDeploymentName] = useSessionStorage(
      SESSION_STORAGE_DEPLOYMENT_NAME_KEY,
      "",
    );
    return {
      id: 0,
      name: storedDeploymentName,
      deploymentType: "dev",
      projectId: 0,
      kind: "local",
      previewIdentifier: null,
    };
  },
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
  enableIndexFilters: true,
};

function DeploymentInfoProvider({
  children,
  deploymentUrl,
  adminKey,
  defaultListDeploymentsApiUrl,
}: {
  children: React.ReactNode;
  deploymentUrl: string | null;
  adminKey: string | null;
  defaultListDeploymentsApiUrl: string | null;
}) {
  const [listDeploymentsApiUrl, setListDeploymentsApiUrl] = useState<
    string | null
  >(null);
  const [selectedDeploymentName, setSelectedDeploymentName] = useState<
    string | null
  >(null);
  const [isValidDeploymentInfo, setIsValidDeploymentInfo] = useState<
    boolean | null
  >(null);
  const [storedAdminKey, setStoredAdminKey] = useSessionStorage("adminKey", "");
  const [storedDeploymentUrl, setStoredDeploymentUrl] = useSessionStorage(
    "deploymentUrl",
    "",
  );
  const [_storedDeploymentName, setStoredDeploymentName] = useSessionStorage(
    SESSION_STORAGE_DEPLOYMENT_NAME_KEY,
    "",
  );
  const onSubmit = useCallback(
    async ({
      submittedAdminKey,
      submittedDeploymentUrl,
      submittedDeploymentName,
    }: {
      submittedAdminKey: string;
      submittedDeploymentUrl: string;
      submittedDeploymentName: string;
    }) => {
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
      setStoredDeploymentName(submittedDeploymentName);
    },
    [setStoredAdminKey, setStoredDeploymentUrl, setStoredDeploymentName],
  );

  useEmbeddedDashboardCredentials(onSubmit);

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
      const listDeploymentsApiPort = url.searchParams.get(
        LIST_DEPLOYMENTS_API_PORT_QUERY_PARAM,
      );
      const selectedDeploymentNameFromUrl = url.searchParams.get(
        SELECTED_DEPLOYMENT_NAME_QUERY_PARAM,
      );
      url.searchParams.delete(LIST_DEPLOYMENTS_API_PORT_QUERY_PARAM);
      url.searchParams.delete(SELECTED_DEPLOYMENT_NAME_QUERY_PARAM);
      window.history.replaceState({}, "", url.toString());
      if (listDeploymentsApiPort) {
        if (!Number.isNaN(parseInt(listDeploymentsApiPort))) {
          setListDeploymentsApiUrl(
            normalizeUrl(`http://127.0.0.1:${listDeploymentsApiPort}`),
          );
        }
      } else {
        setListDeploymentsApiUrl(defaultListDeploymentsApiUrl);
      }
      if (selectedDeploymentNameFromUrl) {
        setSelectedDeploymentName(selectedDeploymentNameFromUrl);
      }
    }
  }, [defaultListDeploymentsApiUrl]);
  if (!mounted) return null;

  if (!isValidDeploymentInfo) {
    return (
      <div className="flex h-screen w-screen flex-col items-center justify-center gap-8">
        <ConvexLogo />
        {listDeploymentsApiUrl !== null ? (
          <DeploymentList
            listDeploymentsApiUrl={listDeploymentsApiUrl}
            onError={() => {
              setListDeploymentsApiUrl(null);
            }}
            onSelect={onSubmit}
            selectedDeploymentName={selectedDeploymentName}
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
          setStoredDeploymentName("");
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

/**
 * Allow the parent window to send credentials to the dashboard.
 * This is used when the dashboard is embedded in another application via an iframe.
 */
function useEmbeddedDashboardCredentials(
  onSubmit: ({
    submittedAdminKey,
    submittedDeploymentUrl,
    submittedDeploymentName,
  }: {
    submittedAdminKey: string;
    submittedDeploymentUrl: string;
    submittedDeploymentName: string;
  }) => void,
) {
  // Send a message to the parent iframe to request the credentials.
  // This prevents race conditions where the parent iframe sends the message
  // before the dashboard loads.
  useEffect(() => {
    window.parent.postMessage(
      {
        type: "dashboard-credentials-request",
      },
      "*",
    );
  }, []);

  useEffect(() => {
    const handleMessage = (event: MessageEvent) => {
      const credentialsSchema = z.object({
        type: z.literal("dashboard-credentials"),
        adminKey: z.string(),
        deploymentUrl: z.string().url(),
        deploymentName: z.string(),
      });

      try {
        credentialsSchema.parse(event.data);
      } catch (err) {
        return;
      }

      if (event.data.type === "dashboard-credentials") {
        onSubmit({
          submittedAdminKey: event.data.adminKey,
          submittedDeploymentUrl: event.data.deploymentUrl,
          submittedDeploymentName: event.data.deploymentName,
        });
      }
    };

    window.addEventListener("message", handleMessage);
    return () => {
      window.removeEventListener("message", handleMessage);
    };
  }, [onSubmit]);
}
