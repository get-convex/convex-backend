import { Context } from "../../../bundler/context.js";
import {
  bigBrainAPI,
  bigBrainAPIMaybeThrows,
  ErrorData,
  logAndHandleFetchError,
  ThrowingFetchError,
} from "../utils/utils.js";
import { components } from "../../generatedApi.js";

/**
 * Verified emails for a user that aren't known to be an admin email for
 * another WorkOS integration.
 */
export async function getCandidateEmailsForWorkIntegration(
  ctx: Context,
): Promise<components["schemas"]["AvailableWorkOSTeamEmailsResponse"]> {
  return bigBrainAPI<
    components["schemas"]["AvailableWorkOSTeamEmailsResponse"]
  >({
    ctx,
    method: "GET",
    url: "workos/available_workos_team_emails",
  });
}

export async function getInvitationEligibleEmails(
  ctx: Context,
  teamId: number,
): Promise<{
  eligibleEmails: string[];
  adminEmail?: string;
}> {
  return bigBrainAPI<{ eligibleEmails: string[]; adminEmail?: string }>({
    ctx,
    method: "GET",
    url: `teams/${teamId}/workos_invitation_eligible_emails`,
  });
}

export async function getDeploymentCanProvisionWorkOSEnvironments(
  ctx: Context,
  deploymentName: string,
): Promise<components["schemas"]["HasAssociatedWorkOSTeamResponse"]> {
  const request: components["schemas"]["HasAssociatedWorkOSTeamRequest"] = {
    deploymentName,
  };
  return bigBrainAPI<components["schemas"]["HasAssociatedWorkOSTeamResponse"]>({
    ctx,
    method: "POST",
    url: "workos/has_associated_workos_team",
    data: request,
  });
}

export async function createEnvironmentAndAPIKey(
  ctx: Context,
  deploymentName: string,
  environmentName?: string,
): Promise<
  | {
      success: true;
      data: components["schemas"]["ProvisionEnvironmentResponse"];
    }
  | {
      success: false;
      error: "team_not_provisioned";
      message: string;
    }
> {
  try {
    const request: components["schemas"]["GetOrProvisionEnvironmentRequest"] = {
      deploymentName,
      environmentName: environmentName ?? null,
    };
    const data = await bigBrainAPI<
      components["schemas"]["ProvisionEnvironmentResponse"]
    >({
      ctx,
      method: "POST",
      url: "workos/get_or_provision_workos_environment",
      data: request,
    });
    return {
      success: true,
      data,
    };
  } catch (error: any) {
    if (error?.message?.includes("WorkOSTeamNotProvisioned")) {
      return {
        success: false,
        error: "team_not_provisioned",
        message: error.message,
      };
    }

    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `Error provisioning WorkOS environment: ${error}`,
    });
  }
}

export async function createAssociatedWorkosTeam(
  ctx: Context,
  teamId: number,
  email: string,
): Promise<
  | {
      result: "success";
      workosTeamId: string;
      workosTeamName: string;
    }
  | {
      result: "emailAlreadyUsed";
      message: string;
    }
> {
  try {
    const request: components["schemas"]["ProvisionWorkOSTeamRequest"] = {
      teamId,
      email,
    };
    const result = (await bigBrainAPIMaybeThrows({
      ctx,
      method: "POST",
      url: "workos/provision_associated_workos_team",
      data: JSON.stringify(request),
    })) as components["schemas"]["ProvisionWorkOSTeamResponse"];
    return {
      result: "success",
      ...result,
    };
  } catch (error) {
    const data: ErrorData | undefined =
      error instanceof ThrowingFetchError ? error.serverErrorData : undefined;
    if (data?.code === "WorkosAccountAlreadyExistsWithThisEmail") {
      return {
        result: "emailAlreadyUsed",
        message:
          data?.message || "WorkOS account with this email already exists",
      };
    }
    return await logAndHandleFetchError(ctx, error);
  }
}

/**
 * Check if the WorkOS team associated with a Convex team is still accessible.
 * Returns null if the team is not provisioned or cannot be accessed.
 */
export async function getWorkosTeamHealth(
  ctx: Context,
  teamId: number,
): Promise<components["schemas"]["WorkOSTeamHealthResponse"] | null> {
  try {
    return (await bigBrainAPIMaybeThrows({
      ctx,
      method: "GET",
      url: `teams/${teamId}/workos_team_health`,
    })) as components["schemas"]["WorkOSTeamHealthResponse"];
  } catch (error: any) {
    if (error?.serverErrorData?.code === "WorkOSTeamNotProvisioned") {
      return null;
    }
    return await logAndHandleFetchError(ctx, error);
  }
}

/**
 * Check if the WorkOS environment associated with a deployment is still accessible.
 * Returns null if the environment is not provisioned or cannot be accessed.
 */
export async function getWorkosEnvironmentHealth(
  ctx: Context,
  deploymentName: string,
): Promise<components["schemas"]["WorkOSEnvironmentHealthResponse"] | null> {
  try {
    return (await bigBrainAPIMaybeThrows({
      ctx,
      method: "GET",
      url: `deployments/${deploymentName}/workos_environment_health`,
    })) as components["schemas"]["WorkOSEnvironmentHealthResponse"];
  } catch (error: any) {
    if (error?.serverErrorData?.code === "WorkOSEnvironmentNotProvisioned") {
      return null;
    }
    return await logAndHandleFetchError(ctx, error);
  }
}

export async function disconnectWorkOSTeam(
  ctx: Context,
  teamId: number,
): Promise<
  | {
      success: true;
      workosTeamId: string;
      workosTeamName: string;
    }
  | {
      success: false;
      error: "not_associated" | "other";
      message: string;
    }
> {
  try {
    const request: components["schemas"]["DisconnectWorkOSTeamRequest"] = {
      teamId,
    };
    const result = (await bigBrainAPIMaybeThrows({
      ctx,
      method: "POST",
      url: "workos/disconnect_workos_team",
      data: JSON.stringify(request),
    })) as components["schemas"]["DisconnectWorkOSTeamResponse"];
    return {
      success: true,
      ...result,
    };
  } catch (error) {
    const data: ErrorData | undefined =
      error instanceof ThrowingFetchError ? error.serverErrorData : undefined;
    if (data?.code === "WorkOSTeamNotAssociated") {
      return {
        success: false,
        error: "not_associated",
        message: data?.message || "No WorkOS team is associated",
      };
    }
    return {
      success: false,
      error: "other",
      message:
        data?.message ||
        (error instanceof Error ? error.message : String(error)),
    };
  }
}

export async function inviteToWorkosTeam(
  ctx: Context,
  teamId: number,
  email: string,
): Promise<
  | {
      result: "success";
      email: string;
      roleSlug: string;
    }
  | {
      result: "teamNotProvisioned";
      message: string;
    }
  | {
      result: "alreadyInWorkspace";
      message: string;
    }
> {
  try {
    const result = await bigBrainAPIMaybeThrows({
      ctx,
      method: "POST",
      url: "workos/invite_team_member",
      data: JSON.stringify({ teamId, email }),
    });
    return { result: "success", ...result };
  } catch (error) {
    const data: ErrorData | undefined =
      error instanceof ThrowingFetchError ? error.serverErrorData : undefined;
    if (data?.code === "WorkOSTeamNotProvisioned") {
      return {
        result: "teamNotProvisioned",
        message: data?.message || "This team doesn't have a WorkOS team yet",
      };
    }
    if (data?.code === "WorkosUserAlreadyInWorkspace") {
      return {
        result: "alreadyInWorkspace",
        message:
          data?.message ||
          "This email is already a member of another WorkOS workspace",
      };
    }
    return await logAndHandleFetchError(ctx, error);
  }
}
