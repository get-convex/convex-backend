import "../styles/globals.css";
import { AppProps } from "next/app";
import Head from "next/head";
import { Favicon } from "elements/Favicon";
import { ThemeConsumer } from "elements/ThemeConsumer";
import {
  DeploymentApiProvider,
  DeploymentDashboardLayout,
  DeploymentInfo,
  DeploymentInfoContext,
  WaitForDeploymentApi,
} from "index";
import { ThemeProvider } from "next-themes";
import { FunctionsProvider } from "../lib/functions/FunctionsProvider";

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
        <DeploymentInfoProvider>
          <DeploymentApiProvider deploymentOverride="local">
            <WaitForDeploymentApi>
              <FunctionsProvider>
                <div className="flex h-screen flex-col">
                  <DeploymentDashboardLayout>
                    <Component {...pageProps} />
                  </DeploymentDashboardLayout>
                </div>
              </FunctionsProvider>
            </WaitForDeploymentApi>
          </DeploymentApiProvider>
        </DeploymentInfoProvider>
      </ThemeProvider>
    </>
  );
}

export const deploymentInfo: DeploymentInfo = {
  ok: true,
  deploymentUrl: process.env.NEXT_PUBLIC_DEPLOYMENT_URL!,
  adminKey: process.env.NEXT_PUBLIC_ADMIN_KEY!,
  useCurrentTeam: () => ({
    id: 1,
    name: "Team",
    slug: "team",
  }),
  useTeamMembers: () => [],
  useCurrentUsageBanner: () => null,
  useCurrentDeployment: () => ({
    id: 1,
    name: "local",
    deploymentType: "prod",
    projectId: 1,
    kind: "local",
  }),
  useHasProjectAdminPermissions: () => true,
  projectsURI: "/",
  deploymentsURI: "/",
};

function DeploymentInfoProvider({ children }: { children: React.ReactNode }) {
  if (deploymentInfo.ok && !deploymentInfo.deploymentUrl) {
    throw new Error("Missing NEXT_PUBLIC_DEPLOYMENT_URL");
  }
  if (deploymentInfo.ok && !deploymentInfo.adminKey) {
    throw new Error("Missing NEXT_PUBLIC_ADMIN_KEY");
  }
  return (
    <DeploymentInfoContext.Provider value={deploymentInfo}>
      {children}
    </DeploymentInfoContext.Provider>
  );
}
