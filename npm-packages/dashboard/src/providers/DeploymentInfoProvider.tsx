import { useEffect, useLayoutEffect, useState } from "react";
import { useRouter } from "next/router";
import {
  captureException,
  captureMessage,
  addBreadcrumb,
  ErrorBoundary,
  FallbackRender,
} from "@sentry/nextjs";
import { reportHttpError } from "hooks/fetching";
import {
  DeploymentInfo,
  DeploymentInfoContext,
} from "@common/lib/deploymentContext";
import { useCurrentTeam, useTeamEntitlements, useTeamMembers } from "api/teams";
import { useCurrentDeployment } from "api/deployments";
import { useHasProjectAdminPermissions } from "api/roles";
import { useCurrentUsageBanner } from "components/header/UsageBanner";
import { useIsDeploymentPaused } from "hooks/useIsDeploymentPaused";
import { CloudImport } from "elements/BackupIdentifier";
import { TeamMemberLink } from "elements/TeamMemberLink";
import { useLogDeploymentEvent } from "hooks/deploymentApi";
import { useAccessToken } from "hooks/useServerSideData";
import { Fallback } from "pages/500";
import { useTeamUsageState } from "api/usage";
import { useProjectEnvironmentVariables } from "api/environmentVariables";
import { useCurrentProject } from "api/projects";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";

// A silly, standard hack to dodge warnings about useLayoutEffect on the server.
const useIsomorphicLayoutEffect =
  typeof window !== "undefined" ? useLayoutEffect : useEffect;

function DeploymentErrorBoundary({
  children,
  fallback,
}: {
  children: React.ReactNode;
  fallback?: React.ReactElement | FallbackRender;
}) {
  return (
    <ErrorBoundary fallback={fallback ?? Fallback}>{children}</ErrorBoundary>
  );
}

export function DeploymentInfoProvider({
  children,
  deploymentOverride,
}: {
  children: React.ReactNode;
  deploymentOverride?: string;
}): JSX.Element {
  const router = useRouter();
  const { deploymentName } = router.query;
  const [deploymentInfo, setDeploymentInfo] = useState<
    DeploymentInfo | undefined
  >(undefined);

  const { newLogsPageSidepanel } = useLaunchDarkly();

  const [accessToken] = useAccessToken();
  const selectedTeamSlug = router.query.team as string;
  const projectSlug = router.query.project as string;
  const teamsURI = `/t/${selectedTeamSlug}`;
  const projectsURI = `${teamsURI}/${projectSlug}`;
  const deploymentsURI = `${projectsURI}/${deploymentName}`;
  useIsomorphicLayoutEffect(() => {
    const f = async () => {
      setDeploymentInfo(undefined);
      const info = await deploymentAuth(
        deploymentOverride || (deploymentName as string),
        `Bearer ${accessToken}`,
      );
      setDeploymentInfo({
        ...info,
        addBreadcrumb,
        captureMessage,
        captureException,
        reportHttpError,
        useCurrentTeam,
        useCurrentProject,
        useCurrentUsageBanner,
        useTeamUsageState,
        useCurrentDeployment,
        useTeamMembers,
        useTeamEntitlements,
        useHasProjectAdminPermissions,
        useProjectEnvironmentVariables,
        useIsDeploymentPaused,
        useLogDeploymentEvent,
        TeamMemberLink,
        CloudImport,
        ErrorBoundary: DeploymentErrorBoundary,
        teamsURI,
        projectsURI,
        deploymentsURI,
        isSelfHosted: false,
        newLogsPageSidepanel,
      });
    };
    if (accessToken && (deploymentOverride || deploymentName)) {
      void f();
    }
  }, [
    accessToken,
    deploymentName,
    deploymentOverride,
    deploymentsURI,
    projectsURI,
    teamsURI,
    newLogsPageSidepanel,
  ]);

  return deploymentInfo ? (
    <DeploymentInfoContext.Provider value={deploymentInfo}>
      {children}
    </DeploymentInfoContext.Provider>
  ) : (
    <>{children}</>
  );
}

const deploymentAuthInner = async (
  deploymentName: string,
  authHeader: string,
  authMethod: string,
): Promise<
  | { deploymentUrl: string; adminKey: string; ok: true }
  | { ok: false; errorMessage: string; errorCode: string }
> => {
  const resp = await fetch(
    `${process.env.NEXT_PUBLIC_BIG_BRAIN_URL}/api/dashboard/instances/${deploymentName}/${authMethod}`,
    {
      method: "POST",
      headers: { Authorization: authHeader },
    },
  );
  const data = await resp.json();
  if (!resp.ok) {
    return { ok: false, errorCode: data.code, errorMessage: data.message };
  }
  const { adminKey, instanceUrl } = data;
  const deploymentUrl = instanceUrl.endsWith("/")
    ? instanceUrl.slice(0, -1)
    : instanceUrl;
  return { deploymentUrl, adminKey, ok: true };
};

// Obtain a deploy key to be displayed to the user for them to use
// in machine based workflows like CI/CD.
const deploymentAuth = async (
  deploymentName: string,
  authHeader: string,
): Promise<
  | { deploymentUrl: string; adminKey: string; ok: true }
  | { ok: false; errorMessage: string; errorCode: string }
> => deploymentAuthInner(deploymentName, authHeader, "auth");
