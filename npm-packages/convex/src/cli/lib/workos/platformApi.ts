import { Context } from "../../../bundler/context.js";
import {
  bigBrainAPI,
  bigBrainAPIMaybeThrows,
  ErrorData,
  logAndHandleFetchError,
  ThrowingFetchError,
} from "../utils/utils.js";

/**
 * Verified emails for a user that aren't known to be an admin email for
 * another WorkOS integration.
 */
export async function getCandidateEmailsForWorkIntegration(
  ctx: Context,
): Promise<{
  availableEmails: string[];
}> {
  return bigBrainAPI<{ availableEmails: string[] }>({
    ctx,
    method: "GET",
    url: "workos/available_workos_team_emails",
  });
}

export async function getDeploymentCanProvisionWorkOSEnvironments(
  ctx: Context,
  deploymentName: string,
): Promise<{
  teamId: number;
  hasAssociatedWorkosTeam: boolean;
  disabled?: boolean;
}> {
  return bigBrainAPI({
    ctx,
    method: "POST",
    url: "workos/has_associated_workos_team",
    data: { deploymentName },
  });
}

export async function createEnvironmentAndAPIKey(
  ctx: Context,
  deploymentName: string,
): Promise<
  | {
      success: true;
      data: {
        environmentId: string;
        environmentName: string;
        clientId: string;
        apiKey: string;
        newlyProvisioned: boolean;
      };
    }
  | {
      success: false;
      error: "team_not_provisioned";
      message: string;
    }
> {
  try {
    const data = await bigBrainAPI({
      ctx,
      method: "POST",
      url: "workos/get_or_provision_workos_environment",
      data: { deploymentName },
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
    const result = await bigBrainAPIMaybeThrows({
      ctx,
      method: "POST",
      url: "workos/provision_associated_workos_team",
      data: JSON.stringify({ teamId, email }),
    });
    return result;
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

export type WorkOSTeamStatus = "Active" | "Inactive";

/**
 * Check if the WorkOS team associated with a Convex team is still accessible.
 * Returns null if the team is not provisioned or cannot be accessed.
 */
export async function getWorkosTeamHealth(
  ctx: Context,
  teamId: number,
): Promise<{
  id: string;
  name: string;
  teamStatus: WorkOSTeamStatus;
} | null> {
  try {
    return await bigBrainAPIMaybeThrows({
      ctx,
      method: "GET",
      url: `teams/${teamId}/workos_team_health`,
    });
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
): Promise<{
  id: string;
  name: string;
  clientId: string;
} | null> {
  try {
    return await bigBrainAPIMaybeThrows({
      ctx,
      method: "GET",
      url: `deployments/${deploymentName}/workos_environment_health`,
    });
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
    const result = await bigBrainAPIMaybeThrows({
      ctx,
      method: "POST",
      url: "workos/disconnect_workos_team",
      data: JSON.stringify({ teamId }),
    });
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
