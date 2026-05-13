import { useEffect, useLayoutEffect, useRef, useState } from "react";
import Link from "next/link";
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
  ConnectedDeployment,
} from "@common/lib/deploymentContext";
import { LocalDeploymentDisconnectOverlay } from "@common/features/disconnectOverlay/LocalDeploymentDisconnectOverlay";
import { CloudDisconnectOverlay } from "@common/features/disconnectOverlay/CloudDisconnectOverlay";
import { useCurrentTeam, useTeamEntitlements, useTeamMembers } from "api/teams";
import { useCurrentDeployment } from "api/deployments";
import { useHasProjectAdminPermissions, useMyCustomRoles } from "api/roles";
import { useProfile } from "api/profile";
import {
  actionResourceKind,
  evaluateRoles,
  type ConcreteResource,
} from "lib/permissions";
import type {
  PlatformDeploymentResponse,
  RoleStatementAction,
} from "@convex-dev/platform/managementApi";
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
import { deploymentAuth } from "lib/deploymentAuth";

// A silly, standard hack to dodge warnings about useLayoutEffect on the server.
const useIsomorphicLayoutEffect =
  typeof window !== "undefined" ? useLayoutEffect : useEffect;

function buildResourceForAction(
  action: RoleStatementAction,
  project: { id: number; slug: string } | undefined,
  deployment: PlatformDeploymentResponse | undefined,
): ConcreteResource | null {
  const kind = actionResourceKind(action);
  switch (kind) {
    case "team":
    case "billing":
    case "sso":
    case "oauthApplication":
    case "integration":
    case "customRole":
      return { segments: [{ kind }] };
    case "project":
      if (!project) return null;
      return {
        segments: [{ kind: "project", id: project.id, slug: project.slug }],
      };
    case "defaultEnvironmentVariable":
      if (!project) return null;
      return {
        segments: [
          { kind: "project", id: project.id, slug: project.slug },
          { kind: "defaultEnvironmentVariable" },
        ],
      };
    case "deployment":
      if (!project || !deployment || deployment.kind !== "cloud") return null;
      return {
        segments: [
          { kind: "project", id: project.id, slug: project.slug },
          {
            kind: "deployment",
            id: deployment.id,
            deploymentType: deployment.deploymentType,
            creator: deployment.creator ?? null,
          },
        ],
      };
    case "member":
    case "token":
      // Member- and token-target actions need IDs the page context
      // doesn't expose; the server returns CustomRoleActionNotImplemented
      // for these today. Deny in the UI to match.
      return null;
    default: {
      const _exhaustive: never = kind;
      return _exhaustive;
    }
  }
}

function useCustomRolePermissionImpl(action: RoleStatementAction): boolean {
  const team = useCurrentTeam();
  const project = useCurrentProject();
  const deployment = useCurrentDeployment();
  const myRoles = useMyCustomRoles(team?.id);
  const profile = useProfile();
  // Local deployments aren't subject to custom-role enforcement (they're
  // single-user dev environments), so don't gate UI on them.
  if (deployment?.kind === "local") {
    return true;
  }
  if (!myRoles || !profile) {
    // While the role list or profile is loading, default to deny so a
    // gated feature doesn't flicker visible-then-hidden on the first
    // render. The profile is needed so `creator=self` selectors resolve.
    return false;
  }
  const resource = buildResourceForAction(action, project, deployment);
  if (!resource) {
    return false;
  }
  return (
    evaluateRoles(myRoles.customRoles, action, resource, profile.id) ===
    "allowed"
  );
}

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
          return deployment;
        },
        useIsProtectedDeployment: () => {
          const deployment = useCurrentDeployment();
          if (!deployment) {
            return false;
          }
          if (deployment.kind === "local") {
            return false;
          }
          return typeof deployment.dashboardEditConfirmation === "boolean"
            ? deployment.dashboardEditConfirmation
            : deployment.deploymentType === "prod";
        },
        useTeamMembers,
        useTeamEntitlements,
        useHasProjectAdminPermissions,
        useCustomRolePermission: useCustomRolePermissionImpl,
        useIsOperationAllowed: () => true,
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
        Link,
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
