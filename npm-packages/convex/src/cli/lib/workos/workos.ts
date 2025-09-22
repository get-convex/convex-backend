/**
 * Programatic provisioning of a WorkOS environments and configuration of these environemnts.
 *
 * This WorkOS integation is subject to change while in development and may require upgrading the CLI
 * to use in the future.
 *
 * This flow may be kicked off by discovering that WORKOS_CLIENT_ID
 * is required in a convex/auth.config.ts but not present on the deployment.
 */
import { Context } from "../../../bundler/context.js";
import {
  changeSpinner,
  logError,
  logFinishedStep,
  logMessage,
  showSpinner,
  stopSpinner,
} from "../../../bundler/log.js";
import { getTeamAndProjectSlugForDeployment } from "../api.js";
import { deploymentDashboardUrlPage } from "../dashboard.js";
import { callUpdateEnvironmentVariables, envGetInDeployment } from "../env.js";
import { changedEnvVarFile, suggestedEnvVarName } from "../envvars.js";
import { promptOptions, promptYesNo } from "../utils/prompts.js";
import { createCORSOrigin, createRedirectURI } from "./environmentApi.js";
import {
  createAssociatedWorkosTeam,
  createEnvironmentAndAPIKey,
  getCandidateEmailsForWorkIntegration,
  getDeploymentCanProvisionWorkOSEnvironments,
} from "./platformApi.js";

/**
 * Ensure the current deployment has the three expected WorkOS environment
 * variables defined with values corresponding to a valid WorkOS deployment.
 *
 * This may involve provisioning a WorkOS deployment or even (in interactive
 * terminals only) prompting to provision a new WorkOS team to be associated
 * with this Convex team.
 */
export async function ensureWorkosEnvironmentProvisioned(
  ctx: Context,
  deploymentName: string,
  deployment: {
    deploymentUrl: string;
    adminKey: string;
    deploymentNotice: string;
  },
): Promise<"ready" | "choseNotToAssociatedTeam"> {
  showSpinner("Checking for associated AuthKit environment...");
  const existingEnvVars = await getExistingWorkosEnvVars(ctx, deployment);
  if (
    existingEnvVars.clientId &&
    existingEnvVars.environmentId &&
    existingEnvVars.apiKey
  ) {
    logFinishedStep(
      "Deployment has a WorkOS environment configured for AuthKit.",
    );
    await updateEnvLocal(ctx, existingEnvVars.clientId);
    await updateWorkosEnvironment(ctx, existingEnvVars.apiKey);
    return "ready";
  }

  // We need to provision an environment. Let's figure out if we can:
  const { hasAssociatedWorkosTeam, teamId } =
    await getDeploymentCanProvisionWorkOSEnvironments(ctx, deploymentName);

  if (!hasAssociatedWorkosTeam) {
    const result = await tryToCreateAssociatedWorkosTeam(
      ctx,
      deploymentName,
      teamId,
    );
    if (result === "choseNotToAssociatedTeam") {
      return "choseNotToAssociatedTeam";
    }
    result satisfies "ready";
  }

  const environmentResult = await createEnvironmentAndAPIKey(
    ctx,
    deploymentName,
  );

  if (!environmentResult.success) {
    if (environmentResult.error === "team_not_provisioned") {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: `Team unexpectedly has no provisioned WorkOS team: ${environmentResult.message}`,
      });
    }
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: environmentResult.message,
    });
  }

  const data = environmentResult.data;
  if (data.newlyProvisioned) {
    logMessage("New AuthKit environment provisioned");
  } else {
    changeSpinner("Using existing AuthKit environment");
  }

  changeSpinner("Setting WORKOS_* deployment environment variables...");
  await setConvexEnvVars(
    ctx,
    deployment,
    data.clientId,
    data.environmentId,
    data.apiKey,
  );
  showSpinner("Updating .env.local with WorkOS configuration");
  await updateEnvLocal(ctx, data.clientId);

  await updateWorkosEnvironment(ctx, data.apiKey);

  return "ready";
}

export async function tryToCreateAssociatedWorkosTeam(
  ctx: Context,
  deploymentName: string,
  teamId: number,
): Promise<"ready" | "choseNotToAssociatedTeam"> {
  const teamInfo = await getTeamAndProjectSlugForDeployment(ctx, {
    deploymentName,
  });
  if (teamInfo === null) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `Can't find Convex Cloud team for deployment ${deploymentName}`,
    });
  }
  stopSpinner();

  const variableName = "WORKOS_CLIENT_ID";
  const variableQuery =
    variableName !== undefined ? `?var=${variableName}` : "";
  const dashboardUrl = deploymentDashboardUrlPage(
    deploymentName,
    `/settings/environment-variables${variableQuery}`,
  );

  const agree = await promptYesNo(ctx, {
    prefix: `A WorkOS team can be created for your Convex team "${teamInfo.teamSlug}" to use for AuthKit.

You and other members of this team will be able to create a WorkOS environments for each Convex deployments for projects on this team.
By creating this account you agree to the WorkOS Terms of Service (https://workos.com/legal/terms-of-service) and Privacy Policy (https://workos.com/legal/privacy).
To provide your own WorkOS environment credentials instead, choose no and set environment variables manually on the dashboard, e.g. \n${dashboardUrl}\n\n`,
    message: `Create WorkOS team and enable automatic AuthKit environment provisioning for team "${teamInfo.teamSlug}"?`,
  });
  if (!agree) {
    return "choseNotToAssociatedTeam";
  }

  const alreadyTried = new Map<string, string>();

  let email;
  while (true) {
    let choice = "refresh";
    while (choice === "refresh") {
      const { availableEmails } =
        await getCandidateEmailsForWorkIntegration(ctx);
      choice = await promptOptions<string>(ctx, {
        message:
          availableEmails.length === 1
            ? "Create a new WorkOS team with this email address?"
            : "Create a new WorkOS team with which email address?",
        suffix:
          availableEmails.length === 0
            ? "\nVisit https://dashboard.convex.dev/profile to add a verified email to use to provision a WorkOS account"
            : availableEmails.length === 1
              ? "\nCreate a new WorkOS team with this email address?"
              : "\nTo use another email address visit https://dashboard.convex.dev/profile to add and verify, then choose 'refresh'",
        choices: [
          ...availableEmails.map((email) => ({
            name: `${email}${alreadyTried.has(email) ? ` (can't create new, already has a WorkOS account)` : ""}`,
            value: email,
          })),
          {
            name: "refresh (add an email at https://dashboard.convex.dev/profile)",
            value: "refresh",
          } as const,
          {
            name: "cancel (do not create a WorkOS account)",
            value: "cancel",
          } as const,
        ],
      });
    }
    if (choice === "cancel") {
      return "choseNotToAssociatedTeam";
    }
    email = choice;

    const teamResult = await createAssociatedWorkosTeam(ctx, teamId, email);

    if (teamResult.result === "emailAlreadyUsed") {
      logMessage(teamResult.message);
      alreadyTried.set(email, teamResult.message);
      continue;
    }
    break;
  }
  logFinishedStep("WorkOS team created successfully");
  return "ready";
}

// Helpers

// In the future this will be configurable.
// Perhaps with an API like `authKit({ redirectUri: 'asdf' })
async function updateWorkosEnvironment(
  ctx: Context,
  workosApiKey: string,
): Promise<void> {
  let { frontendDevUrl } = await suggestedEnvVarName(ctx);
  frontendDevUrl = frontendDevUrl || "http://localhost:5173";
  const redirectUri = `${frontendDevUrl}/callback`;
  const corsOrigin = `${frontendDevUrl}`;

  await applyConfigToWorkosEnvironment(ctx, {
    workosApiKey,
    redirectUri,
    corsOrigin,
  });
}

async function applyConfigToWorkosEnvironment(
  ctx: Context,
  {
    workosApiKey,
    redirectUri,
    corsOrigin,
  }: {
    workosApiKey: string;
    redirectUri: string;
    corsOrigin: string;
  },
): Promise<void> {
  changeSpinner("Configuring AuthKit redirect URI...");
  const { modified: redirectUriAdded } = await createRedirectURI(
    ctx,
    workosApiKey,
    redirectUri,
  );
  if (redirectUriAdded) {
    logMessage(`AuthKit redirect URI added: ${redirectUri}`);
  }

  changeSpinner("Configuring AuthKit CORS origin...");
  const { modified: corsAdded } = await createCORSOrigin(
    ctx,
    workosApiKey,
    corsOrigin,
  );
  if (corsAdded) {
    logMessage(`AuthKit CORS origin added: ${corsOrigin}`);
  }
}

async function updateEnvLocal(ctx: Context, clientId: string) {
  const envPath = ".env.local";

  try {
    const existingContent = ctx.fs.exists(envPath)
      ? ctx.fs.readUtf8File(envPath)
      : null;

    const clientIdUpdate = changedEnvVarFile({
      existingFileContent: existingContent,
      envVarName: "VITE_WORKOS_CLIENT_ID",
      envVarValue: clientId,
      commentAfterValue: null,
      commentOnPreviousLine: null,
    });

    if (clientIdUpdate !== null) {
      ctx.fs.writeUtf8File(envPath, clientIdUpdate);
    }

    const redirectUriUpdate = changedEnvVarFile({
      existingFileContent: clientIdUpdate || existingContent,
      envVarName: "VITE_WORKOS_REDIRECT_URI",
      envVarValue: "http://localhost:5173/callback",
      commentAfterValue: null,
      commentOnPreviousLine: null,
    });

    if (redirectUriUpdate !== null) {
      ctx.fs.writeUtf8File(envPath, redirectUriUpdate);
    }
  } catch (error) {
    logError(`Could not update .env.local: ${String(error)}`);
  }
}

async function getExistingWorkosEnvVars(
  ctx: Context,
  deployment: {
    deploymentUrl: string;
    adminKey: string;
  },
): Promise<{
  clientId: string | null;
  environmentId: string | null;
  apiKey: string | null;
}> {
  const [clientId, environmentId, apiKey] = await Promise.all([
    envGetInDeployment(ctx, deployment, "WORKOS_CLIENT_ID"),
    envGetInDeployment(ctx, deployment, "WORKOS_ENVIRONMENT_ID"),
    envGetInDeployment(ctx, deployment, "WORKOS_ENVIRONMENT_API_KEY"),
  ]);

  return { clientId, environmentId, apiKey };
}

async function setConvexEnvVars(
  ctx: Context,
  deployment: {
    deploymentUrl: string;
    adminKey: string;
    deploymentNotice: string;
  },
  workosClientId: string,
  workosEnvironmentId: string,
  workosEnvironmentApiKey: string,
) {
  await callUpdateEnvironmentVariables(ctx, deployment, [
    { name: "WORKOS_CLIENT_ID", value: workosClientId },
    { name: "WORKOS_ENVIRONMENT_ID", value: workosEnvironmentId },
    { name: "WORKOS_ENVIRONMENT_API_KEY", value: workosEnvironmentApiKey },
  ]);
}
