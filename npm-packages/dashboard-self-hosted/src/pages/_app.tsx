// eslint-disable-next-line import/no-relative-packages
import "../../../dashboard-common/src/styles/globals.css";
import { AppProps } from "next/app";
import Head from "next/head";
import { useQuery } from "convex/react";
import udfs from "dashboard-common/udfs";
import { useSessionStorage } from "react-use";
import {
  EnterIcon,
  ExitIcon,
  EyeNoneIcon,
  EyeOpenIcon,
  GearIcon,
} from "@radix-ui/react-icons";
import { ConvexLogo } from "dashboard-common/elements/ConvexLogo";
import { ToastContainer } from "dashboard-common/elements/ToastContainer";
import { ThemeConsumer } from "dashboard-common/elements/ThemeConsumer";
import { Favicon } from "dashboard-common/elements/Favicon";
import { ToggleTheme } from "dashboard-common/elements/ToggleTheme";
import { Menu, MenuItem } from "dashboard-common/elements/Menu";
import { TextInput } from "dashboard-common/elements/TextInput";
import { Button } from "dashboard-common/elements/Button";
import { ThemeProvider } from "next-themes";
import React, { useEffect, useMemo, useState } from "react";
import { ErrorBoundary } from "components/ErrorBoundary";
import { DeploymentDashboardLayout } from "dashboard-common/layouts/DeploymentDashboardLayout";
import {
  DeploymentApiProvider,
  WaitForDeploymentApi,
  DeploymentInfo,
  DeploymentInfoContext,
} from "dashboard-common/lib/deploymentContext";

function App({
  Component,
  pageProps: { deploymentUrl, ...pageProps },
}: AppProps & { pageProps: { deploymentUrl: string } }) {
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
          <DeploymentInfoProvider deploymentUrl={deploymentUrl}>
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

App.getInitialProps = async ({ ctx }: { ctx: { req?: any } }) => {
  // On server-side, get from process.env
  if (ctx.req) {
    const deploymentUrl = process.env.NEXT_PUBLIC_DEPLOYMENT_URL;
    if (!deploymentUrl) {
      throw new Error(
        "NEXT_PUBLIC_DEPLOYMENT_URL environment variable is not set",
      );
    }
    return {
      pageProps: {
        deploymentUrl,
      },
    };
  }

  // On client-side navigation, get from window.__NEXT_DATA__
  const deploymentUrl = window.__NEXT_DATA__?.props?.pageProps?.deploymentUrl;
  if (!deploymentUrl) {
    throw new Error("deploymentUrl not found in __NEXT_DATA__");
  }

  return {
    pageProps: {
      deploymentUrl,
    },
  };
};

export default App;

const deploymentInfo: Omit<DeploymentInfo, "deploymentUrl" | "adminKey"> = {
  ok: true,
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
  TeamMemberLink: () => <div />,
  ErrorBoundary: ({ children }: { children: React.ReactNode }) => (
    <ErrorBoundary>{children}</ErrorBoundary>
  ),
  useTeamUsageState: () => "Default",
  teamsURI: "",
  projectsURI: "",
  deploymentsURI: "",
  isSelfHosted: true,
};

function DeploymentInfoProvider({
  children,
  deploymentUrl,
}: {
  children: React.ReactNode;
  deploymentUrl: string;
}) {
  const [adminKey, setAdminKey] = useSessionStorage("adminKey", "");
  const [draftAdminKey, setDraftAdminKey] = useState<string>("");

  const [showKey, setShowKey] = useState(false);

  const finalValue: DeploymentInfo = useMemo(
    () =>
      ({
        ...deploymentInfo,
        ok: true,
        adminKey,
        deploymentUrl,
      }) as DeploymentInfo,
    [adminKey, deploymentUrl],
  );
  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);
  if (!mounted) return null;

  if (!adminKey) {
    return (
      <div className="flex h-screen w-screen flex-col items-center justify-center gap-8">
        <ConvexLogo />
        <form
          className="flex w-[30rem] flex-col gap-2"
          onSubmit={() => {
            setDraftAdminKey("");
            setAdminKey(draftAdminKey);
          }}
        >
          <TextInput
            id="adminKey"
            label="Admin Key"
            type={showKey ? "text" : "password"}
            Icon={showKey ? EyeNoneIcon : EyeOpenIcon}
            outerClassname="w-[30rem]"
            placeholder="Enter the admin key for this deployment"
            value={draftAdminKey}
            action={() => {
              setShowKey(!showKey);
            }}
            description="The admin key is required every time you open the dashboard."
            onChange={(e) => {
              setDraftAdminKey(e.target.value);
            }}
          />
          <Button
            type="submit"
            icon={<EnterIcon />}
            disabled={!draftAdminKey}
            size="xs"
            className="ml-auto w-fit"
          >
            Log In
          </Button>
        </form>
      </div>
    );
  }
  return (
    <DeploymentInfoContext.Provider value={finalValue}>
      <ErrorBoundary>
        <Header onLogout={() => setAdminKey("")} />
        {children}
      </ErrorBoundary>
    </DeploymentInfoContext.Provider>
  );
}

function Header({ onLogout }: { onLogout: () => void }) {
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
