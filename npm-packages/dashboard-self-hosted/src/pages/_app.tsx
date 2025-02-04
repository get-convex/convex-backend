// eslint-disable-next-line import/no-relative-packages
import "../../../dashboard-common/src/styles/globals.css";
import { AppProps } from "next/app";
import Head from "next/head";
import { useQuery } from "convex/react";
import udfs from "udfs";
import { useSessionStorage } from "react-use";
import {
  EnterIcon,
  ExitIcon,
  EyeNoneIcon,
  EyeOpenIcon,
  GearIcon,
} from "@radix-ui/react-icons";
import {
  ConvexLogo,
  DeploymentApiProvider,
  DeploymentInfo,
  DeploymentInfoContext,
  WaitForDeploymentApi,
  ToastContainer,
  DeploymentDashboardLayout,
  ThemeConsumer,
  Favicon,
  ThemeProvider,
  ToggleTheme,
  Menu,
  MenuItem,
  TextInput,
  Button,
  Sheet,
} from "dashboard-common";
import React, {
  ErrorInfo,
  ReactNode,
  useEffect,
  useMemo,
  useState,
} from "react";

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
  useTeamUsageState: () => "Default",
  teamsURI: "/",
  projectsURI: "/",
  deploymentsURI: "/",
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

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  error?: Error;
}

class ErrorBoundary extends React.Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = {};
  }

  static getDerivedStateFromError(e: Error): ErrorBoundaryState {
    return { error: e };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("Uncaught error:", error, errorInfo);
  }

  render() {
    const { error } = this.state;
    const { children } = this.props;
    if (error) {
      return (
        <div className="flex h-screen w-screen flex-col items-center justify-center gap-4">
          <h3>Something went wrong</h3>
          <div className="flex flex-col items-center gap-2">
            {error.message.includes("not permitted") && (
              <p role="alert" className="text-sm">
                Your admin key may be invalid. Please try logging in again.
              </p>
            )}
            <Button
              className="w-fit"
              icon={<ExitIcon />}
              size="xs"
              onClick={() => {
                window.sessionStorage.setItem("adminKey", "");
                window.location.reload();
              }}
              variant="neutral"
            >
              Log Out
            </Button>
          </div>
          <Sheet className="max-h-[50vh] w-[50rem] max-w-[80vw] overflow-auto font-mono text-sm">
            {error.message}
            <pre>
              <code>{error.stack}</code>
            </pre>
          </Sheet>
        </div>
      );
    }

    return children;
  }
}
