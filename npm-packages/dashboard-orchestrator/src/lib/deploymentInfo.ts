// Stub helpers and constants for building a `DeploymentInfo` value the
// shared dashboard-common deployment-content components consume.

import type { DeploymentInfo } from "@common/lib/deploymentContext";
import type { PlatformDeploymentResponse } from "@convex-dev/platform/managementApi";

/**
 * Hosted-only operations the orchestrator doesn't implement. Returned in
 * every DeploymentInfo so dashboard-common components don't crash.
 */
export const stubWorkOsOperations: DeploymentInfo["workOSOperations"] = {
  useDeploymentWorkOSEnvironment: () => ({ data: undefined, error: undefined }),
  useTeamWorkOSIntegration: () => undefined,
  useWorkOSTeamHealth: () => undefined,
  useWorkOSEnvironmentHealth: () => ({ data: undefined, error: undefined }),
  useDisconnectWorkOSTeam: () => async () => undefined,
  useInviteWorkOSTeamMember: () => async () => undefined,
  useWorkOSInvitationEligibleEmails: () => undefined,
  useAvailableWorkOSTeamEmails: () => undefined,
  useProvisionWorkOSTeam: () => async () => undefined,
  useProvisionWorkOSEnvironment: () => async () => undefined,
  useDeleteWorkOSEnvironment: () => async () => undefined,
  useProjectWorkOSEnvironments: () => undefined,
  useGetProjectWorkOSEnvironment: () => undefined,
  useCheckProjectEnvironmentHealth: () => async () => null,
  useProvisionProjectWorkOSEnvironment: () => async () => ({
    workosEnvironmentId: "",
    workosEnvironmentName: "",
    workosClientId: "",
    workosApiKey: "",
    newlyProvisioned: false,
    userEnvironmentName: "",
  }),
  useDeleteProjectWorkOSEnvironment: () => async () => ({
    workosEnvironmentId: "",
    workosEnvironmentName: "",
    workosTeamId: "",
  }),
};

export type ResolvedDeployment = {
  team: { id: number; name: string; slug: string };
  project: { id: number; name: string; slug: string };
  deployment: PlatformDeploymentResponse;
  adminKey: string;
  deploymentUrl: string;
};
