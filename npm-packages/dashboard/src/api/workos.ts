import { mutate as globalMutate } from "swr";
import type {
  DeleteWorkOsEnvironmentRequest,
  DisconnectWorkOsTeamRequest,
  GetOrProvisionEnvironmentRequest,
  ProvisionWorkOsTeamRequest,
} from "generatedApi";
import { useBBMutation, useBBQuery } from "./api";

// Helper function to invalidate all WorkOS-related cache keys for a team
async function invalidateWorkOSTeamCache(teamId: string) {
  await globalMutate(
    (key) => {
      if (!Array.isArray(key) || key.length < 3) return false;
      const [prefix, keyPath] = key;
      if (prefix !== "big-brain") return false;

      // Invalidate any WorkOS-related endpoints for this team
      // Also invalidate deployment-level endpoints since they include team data
      return (
        keyPath === `/teams/${teamId}/workos_integration` ||
        keyPath === `/teams/${teamId}/workos_team_health` ||
        keyPath === `/teams/${teamId}/workos_invitation_eligible_emails` ||
        keyPath === "/workos/available_workos_team_emails" ||
        (keyPath.startsWith("/deployments/") &&
          keyPath.includes("/workos_environment"))
      );
    },
    undefined,
    { revalidate: true },
  );
}

// Helper function to invalidate all WorkOS-related cache keys for a deployment
async function invalidateWorkOSEnvironmentCache(deploymentName: string) {
  await globalMutate(
    (key) => {
      if (!Array.isArray(key) || key.length < 3) return false;
      const [prefix, keyPath] = key;
      if (prefix !== "big-brain") return false;

      // Invalidate any WorkOS-related endpoints for this deployment
      return (
        keyPath === `/deployments/${deploymentName}/workos_environment` ||
        keyPath === `/deployments/${deploymentName}/workos_environment_health`
      );
    },
    undefined,
    { revalidate: true },
  );
}

export function useDeploymentWorkOSEnvironment(deploymentName?: string) {
  const { data } = useBBQuery({
    path: "/deployments/{deployment_name}/workos_environment",
    pathParams: { deployment_name: deploymentName || "" },
  });

  return data;
}

export function useTeamWorkOSIntegration(teamId?: string) {
  const { data } = useBBQuery({
    path: "/teams/{team_id}/workos_integration",
    pathParams: { team_id: teamId || "" },
  });
  return data;
}

export function useWorkOSTeamHealth(teamId?: string) {
  const { data, error } = useBBQuery({
    path: "/teams/{team_id}/workos_team_health",
    pathParams: { team_id: teamId || "" },
  });
  return { data, error };
}

export function useWorkOSEnvironmentHealth(deploymentName?: string) {
  const { data, error } = useBBQuery({
    path: "/deployments/{deployment_name}/workos_environment_health",
    pathParams: { deployment_name: deploymentName || "" },
  });
  return { data, error };
}

export function useDisconnectWorkOSTeam(teamId?: string) {
  const mutation = useBBMutation({
    path: "/workos/disconnect_workos_team",
    pathParams: undefined,
    method: "post",
    successToast: "WorkOS team disconnected successfully",
    mutateKey: "/teams/{team_id}/workos_integration",
    mutatePathParams: { team_id: teamId || "" },
  });

  return async (body: DisconnectWorkOsTeamRequest) => {
    const result = await mutation(body);
    // Invalidate all related WorkOS caches for this team
    if (teamId) {
      await invalidateWorkOSTeamCache(teamId);
    }
    return result;
  };
}

export function useInviteWorkOSTeamMember() {
  return useBBMutation({
    path: "/workos/invite_team_member",
    pathParams: undefined,
    method: "post",
    successToast: "WorkOS invitation sent successfully",
    toastOnError: false,
  });
}

export function useWorkOSInvitationEligibleEmails(teamId?: string) {
  const { data } = useBBQuery({
    path: "/teams/{team_id}/workos_invitation_eligible_emails",
    pathParams: { team_id: teamId || "" },
  });
  return data;
}

export function useAvailableWorkOSTeamEmails() {
  const { data } = useBBQuery({
    path: "/workos/available_workos_team_emails",
    pathParams: undefined,
  });
  return data;
}

export function useProvisionWorkOSTeam(teamId?: string) {
  const mutation = useBBMutation({
    path: "/workos/provision_associated_workos_team",
    pathParams: undefined,
    method: "post",
    successToast: "WorkOS team created successfully",
    mutateKey: "/teams/{team_id}/workos_integration",
    mutatePathParams: { team_id: teamId || "" },
  });

  return async (body: ProvisionWorkOsTeamRequest) => {
    const result = await mutation(body);
    // Invalidate all related WorkOS caches for this team
    if (teamId) {
      await invalidateWorkOSTeamCache(teamId);
    }
    return result;
  };
}

export function useProvisionWorkOSEnvironment(deploymentName?: string) {
  const mutation = useBBMutation({
    path: "/workos/get_or_provision_workos_environment",
    pathParams: undefined,
    method: "post",
    successToast: "WorkOS environment provisioned successfully",
    mutateKey: "/deployments/{deployment_name}/workos_environment",
    mutatePathParams: { deployment_name: deploymentName || "" },
  });

  return async (body: GetOrProvisionEnvironmentRequest) => {
    const result = await mutation(body);
    // Invalidate all related WorkOS caches for this deployment
    if (deploymentName) {
      await invalidateWorkOSEnvironmentCache(deploymentName);
    }
    return result;
  };
}

export function useDeleteWorkOSEnvironment(deploymentName?: string) {
  const mutation = useBBMutation({
    path: "/workos/delete_environment",
    pathParams: undefined,
    method: "post",
    successToast: "WorkOS environment deleted successfully",
    mutateKey: "/deployments/{deployment_name}/workos_environment",
    mutatePathParams: { deployment_name: deploymentName || "" },
  });

  return async (body: DeleteWorkOsEnvironmentRequest) => {
    const result = await mutation(body);
    // Invalidate all related WorkOS caches for this deployment
    if (deploymentName) {
      await invalidateWorkOSEnvironmentCache(deploymentName);
    }
    return result;
  };
}
