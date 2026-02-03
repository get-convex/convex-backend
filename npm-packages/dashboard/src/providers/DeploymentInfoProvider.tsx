import { useEffect, useLayoutEffect, useRef, useState } from "react";
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
  LocalDeploymentDisconnectOverlay,
  CloudDisconnectOverlay,
  ConnectedDeployment,
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
import {
  useDeploymentWorkOSEnvironment,
  useTeamWorkOSIntegration,
  useWorkOSTeamHealth,
  useWorkOSEnvironmentHealth,
  useDisconnectWorkOSTeam,
  useInviteWorkOSTeamMember,
  useWorkOSInvitationEligibleEmails,
  useAvailableWorkOSTeamEmails,
  useProvisionWorkOSTeam,
  useProvisionWorkOSEnvironment,
  useDeleteWorkOSEnvironment,
  useProjectWorkOSEnvironments,
  useGetProjectWorkOSEnvironment,
  useCheckProjectEnvironmentHealth,
  useProvisionProjectWorkOSEnvironment,
  useDeleteProjectWorkOSEnvironment,
} from "api/workos";
import { useSupportFormOpen } from "elements/SupportWidget";
import { useConvexStatus } from "hooks/useConvexStatus";
import { ConvexStatusWidget } from "lib/ConvexStatusWidget";

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

function CloudDashboardDisconnectOverlay({
  deployment,
  deploymentName,
}: {
  deployment: ConnectedDeployment;
  deploymentName: string;
}) {
  const [, setOpenState] = useSupportFormOpen();
  const { status } = useConvexStatus();

  const openSupportForm = (defaultSubject: string, defaultMessage: string) => {
    setOpenState({
      defaultSubject,
      defaultMessage,
    });
  };

  if (deploymentName.startsWith("local-")) {
    return <LocalDeploymentDisconnectOverlay />;
  }

  return (
    <CloudDisconnectOverlay
      deployment={deployment}
      deploymentName={deploymentName}
      openSupportForm={openSupportForm}
      statusWidget={
        <>
          <ConvexStatusWidget status={status} />
          {status?.indicator === "none" && (
            <p className="mt-2 text-xs text-content-secondary">
              For emerging issues, it may take the Convex team a few minutes to
              update system status.
            </p>
          )}
        </>
      }
    />
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

  const [accessToken] = useAccessToken();
  // Use a ref to track the access token without triggering re-authentication
  // when it changes
  const accessTokenRef = useRef(accessToken);
  useEffect(() => {
    accessTokenRef.current = accessToken;
  }, [accessToken]);
  const {
    workOsEnvironmentProvisioningDashboardUi,
    connectionStateCheckIntervalMs,
  } = useLaunchDarkly();
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
        `Bearer ${accessTokenRef.current}`,
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
        useCurrentDeployment: () => {
          const deployment = useCurrentDeployment();
          if (!deployment) return undefined;
          // Map PlatformDeploymentResponse to the expected type
          // Local deployments don't have an id in the API, so we use 0 as a fallback
          return {
            id: deployment.kind === "cloud" ? deployment.id : 0,
            name: deployment.name,
            projectId: deployment.projectId,
            deploymentType: deployment.deploymentType,
            kind: deployment.kind,
            previewIdentifier: deployment.previewIdentifier,
          };
        },
        useIsProtectedDeployment: () => {
          const deployment = useCurrentDeployment();
          return deployment?.deploymentType === "prod";
        },
        useTeamMembers,
        useTeamEntitlements,
        useHasProjectAdminPermissions,
        useProjectEnvironmentVariables,
        useIsDeploymentPaused,
        useLogDeploymentEvent,
        workOSOperations: {
          useDeploymentWorkOSEnvironment,
          useTeamWorkOSIntegration,
          useWorkOSTeamHealth,
          useWorkOSEnvironmentHealth,
          useDisconnectWorkOSTeam,
          useInviteWorkOSTeamMember,
          useWorkOSInvitationEligibleEmails,
          useAvailableWorkOSTeamEmails,
          useProvisionWorkOSTeam,
          useProvisionWorkOSEnvironment,
          useDeleteWorkOSEnvironment,
          useProjectWorkOSEnvironments,
          useGetProjectWorkOSEnvironment,
          useCheckProjectEnvironmentHealth,
          useProvisionProjectWorkOSEnvironment,
          useDeleteProjectWorkOSEnvironment,
        },
        TeamMemberLink,
        CloudImport,
        ErrorBoundary: DeploymentErrorBoundary,
        DisconnectOverlay: CloudDashboardDisconnectOverlay,
        teamsURI,
        projectsURI,
        deploymentsURI,
        isSelfHosted: false,
        workosIntegrationEnabled: workOsEnvironmentProvisioningDashboardUi,
        connectionStateCheckIntervalMs,
      });
    };
    if (accessTokenRef.current && (deploymentOverride || deploymentName)) {
      void f();
    }
  }, [
    // Note: accessToken is intentionally NOT in dependencies
    // We don't want to re-authenticate to the deployment every time the dashboard
    // access token refreshes (every 10 minutes). The deployment admin key is separate
    // and doesn't need to be refreshed when the dashboard token changes.
    deploymentName,
    deploymentOverride,
    deploymentsURI,
    projectsURI,
    teamsURI,
    workOsEnvironmentProvisioningDashboardUi,
    connectionStateCheckIntervalMs,
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
