// eslint-disable-next-line import/no-relative-packages
import "../../../dashboard-common/src/styles/globals.css";
import { AppProps } from "next/app";
import Head from "next/head";
import { useQuery } from "convex/react";
import udfs from "udfs";
import {
  DeploymentApiProvider,
  DeploymentInfo,
  DeploymentInfoContext,
  WaitForDeploymentApi,
  ToastContainer,
  DeploymentDashboardLayout,
  ThemeConsumer,
  Favicon,
  ThemeProvider,
} from "dashboard-common";

export default function App({ Component, pageProps }: AppProps) {
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
        <DeploymentInfoProvider>
          <DeploymentApiProvider deploymentOverride="local">
            <WaitForDeploymentApi>
              <div className="flex h-screen flex-col">
                <DeploymentDashboardLayout>
                  <Component {...pageProps} />
                </DeploymentDashboardLayout>
              </div>
            </WaitForDeploymentApi>
          </DeploymentApiProvider>
        </DeploymentInfoProvider>
      </ThemeProvider>
    </>
  );
}

const deploymentInfo: DeploymentInfo = {
  ok: true,
  deploymentUrl: process.env.NEXT_PUBLIC_DEPLOYMENT_URL!,
  adminKey: process.env.NEXT_PUBLIC_ADMIN_KEY!,
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

function DeploymentInfoProvider({ children }: { children: React.ReactNode }) {
  return (
    <DeploymentInfoContext.Provider value={deploymentInfo}>
      {children}
    </DeploymentInfoContext.Provider>
  );
}
