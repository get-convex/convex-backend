/**
 * Debugging commands for the WorkOS integration; these are unstable, undocumented, and will change or disappear as the WorkOS integration evolves.
 **/
import { Command } from "@commander-js/extra-typings";
import { Context, oneoffContext } from "../bundler/context.js";
import { chalkStderr } from "chalk";
import {
  DeploymentSelectionOptions,
  deploymentSelectionWithinProjectFromOptions,
  fetchTeamAndProject,
  getTeamAndProjectSlugForDeployment,
  loadSelectedDeploymentCredentials,
} from "./lib/api.js";
import { actionDescription } from "./lib/command.js";
import { ensureHasConvexDependency } from "./lib/utils/utils.js";
import { getDeploymentSelection } from "./lib/deploymentSelection.js";
import {
  ensureWorkosEnvironmentProvisioned,
  provisionWorkosTeamInteractive,
} from "./lib/workos/workos.js";
import {
  disconnectWorkOSTeam,
  getCandidateEmailsForWorkIntegration,
  getDeploymentCanProvisionWorkOSEnvironments,
  getInvitationEligibleEmails,
  getWorkosEnvironmentHealth,
  getWorkosTeamHealth,
  inviteToWorkosTeam,
  listProjectWorkOSEnvironments,
  createProjectWorkOSEnvironment,
  deleteProjectWorkOSEnvironment,
} from "./lib/workos/platformApi.js";
import {
  logFinishedStep,
  logMessage,
  logWarning,
  showSpinner,
  stopSpinner,
} from "../bundler/log.js";
import { readProjectConfig, getAuthKitConfig } from "./lib/config.js";
import { promptOptions, promptYesNo } from "./lib/utils/prompts.js";

async function selectEnvDeployment(
  options: DeploymentSelectionOptions,
): Promise<{
  ctx: Context;
  deployment: {
    deploymentUrl: string;
    deploymentName: string;
    deploymentType: "dev" | "preview" | "prod";
    adminKey: string;
    deploymentNotice: string;
  };
}> {
  const ctx = await oneoffContext(options);
  const deploymentSelection = await getDeploymentSelection(ctx, options);
  const selectionWithinProject =
    deploymentSelectionWithinProjectFromOptions(options);
  const {
    adminKey,
    url: deploymentUrl,
    deploymentFields,
  } = await loadSelectedDeploymentCredentials(
    ctx,
    deploymentSelection,
    selectionWithinProject,
  );
  // WorkOS integration only works with cloud deployments
  if (!deploymentFields) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: "WorkOS integration requires a configured deployment",
    });
  }

  const deploymentNotice = ` (on ${chalkStderr.bold(deploymentFields.deploymentType)} deployment ${chalkStderr.bold(deploymentFields.deploymentName)})`;

  const deploymentType = deploymentFields.deploymentType;

  if (deploymentType === "custom") {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `The WorkOS integration is not available for custom deployments yet.`,
    });
  }

  if (
    deploymentType !== "dev" &&
    deploymentType !== "preview" &&
    deploymentType !== "prod"
  ) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `The WorkOS integration is only available for cloud deployments (dev, preview, prod), not ${deploymentType}`,
    });
  }

  // Now TypeScript knows deploymentType is CloudDeploymentType
  return {
    ctx,
    deployment: {
      deploymentName: deploymentFields.deploymentName,
      deploymentType,
      deploymentUrl,
      adminKey,
      deploymentNotice,
    },
  };
}

const workosTeamStatus = new Command("status")
  .summary("Status of associated WorkOS team and environment")
  .addDeploymentSelectionOptions(actionDescription("Check WorkOS status for"))
  .action(async (_options, cmd) => {
    const options = cmd.optsWithGlobals();
    const { ctx, deployment } = await selectEnvDeployment(options);

    const info = await fetchTeamAndProject(ctx, deployment.deploymentName);

    // Check team status
    const teamHealth = await getWorkosTeamHealth(ctx, info.teamId);
    if (!teamHealth) {
      logMessage(`WorkOS team: Not provisioned`);
      const { availableEmails } =
        await getCandidateEmailsForWorkIntegration(ctx);
      if (availableEmails.length > 0) {
        logMessage(
          `  Verified emails that can provision: ${availableEmails.join(", ")}`,
        );
      }
    } else if (teamHealth.productionState === "inactive") {
      logMessage(
        `WorkOS team: ${teamHealth.name} (no credit card added on workos.com, so production auth environments cannot be created)`,
      );
    } else {
      logMessage(`WorkOS team: ${teamHealth.name}`);
    }

    // Check environment status
    const envHealth = await getWorkosEnvironmentHealth(
      ctx,
      deployment.deploymentName,
    );
    if (!envHealth) {
      logMessage(`WorkOS environment: Not provisioned`);
    } else {
      logMessage(`WorkOS environment: ${envHealth.name}`);
      const workosUrl = `https://dashboard.workos.com/${envHealth.id}/authentication`;
      logMessage(`${workosUrl}`);
    }

    try {
      const { projectConfig } = await readProjectConfig(ctx);
      const authKitConfig = await getAuthKitConfig(ctx, projectConfig);

      if (!authKitConfig) {
        logMessage(
          `AuthKit config: ${chalkStderr.dim("Not configured in convex.json")}`,
        );
      } else {
        logMessage(`AuthKit config:`);

        // Show config for each deployment type
        for (const deploymentType of ["dev", "preview", "prod"] as const) {
          const envConfig = authKitConfig[deploymentType];
          if (!envConfig) {
            logMessage(
              `  ${deploymentType}: ${chalkStderr.dim("not configured")}`,
            );
            continue;
          }

          // Build description based on what's configured
          let description = "";

          // Show environment type for prod deployments
          if (deploymentType === "prod" && envConfig.environmentType) {
            description = `environment type: ${envConfig.environmentType}`;
          }

          const configureStatus =
            envConfig.configure === false
              ? ", configure: disabled"
              : envConfig.configure
                ? ", will configure WorkOS"
                : "";

          const localEnvVarsStatus =
            envConfig.localEnvVars === false
              ? ""
              : envConfig.localEnvVars
                ? `, ${Object.keys(envConfig.localEnvVars).length} local env vars`
                : "";

          // Show deployment type with its configuration
          const configInfo = [description, configureStatus, localEnvVarsStatus]
            .filter((s) => s)
            .join("");

          logMessage(`  ${deploymentType}: ${configInfo || "configured"}`);
        }
      }
    } catch (error) {
      logMessage(
        `AuthKit config: ${chalkStderr.yellow(`Error reading config: ${String(error)}`)}`,
      );
    }
  });

const workosProvisionEnvironment = new Command("provision-environment")
  .summary("Provision a WorkOS environment")
  .description(
    "Create or get the WorkOS environment and API key for this deployment",
  )
  .configureHelp({ showGlobalOptions: true })
  .allowExcessArguments(false)
  .addDeploymentSelectionOptions(
    actionDescription("Provision WorkOS environment for"),
  )
  .option(
    "--name <name>",
    "Custom name for the WorkOS environment (if not provided, uses deployment name)",
  )
  .action(async (_options, cmd) => {
    const options = cmd.optsWithGlobals();
    const { ctx, deployment } = await selectEnvDeployment(options);
    await ensureHasConvexDependency(
      ctx,
      "integration workos provision-environment",
    );

    try {
      const { projectConfig } = await readProjectConfig(ctx);
      const authKitConfig = await getAuthKitConfig(ctx, projectConfig);
      const config = authKitConfig || { dev: {} };

      if (!authKitConfig) {
        logWarning(
          "Consider using the 'authKit' config in convex.json for automatic provisioning.",
        );
        logMessage(
          "Learn more at https://docs.convex.dev/auth/authkit/auto-provision",
        );
        logMessage("");
      }

      await ensureWorkosEnvironmentProvisioned(
        ctx,
        deployment.deploymentName,
        deployment,
        config,
        deployment.deploymentType,
      );
    } catch (error) {
      await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        errForSentry: error,
        printedMessage: `Failed to provision WorkOS environment: ${String(error)}`,
      });
    }
  });

const workosProvisionTeam = new Command("provision-team")
  .summary("Provision a WorkOS team for this Convex team")
  .description(
    "Create a WorkOS team and associate it with this Convex team. " +
      "This enables automatic provisioning of WorkOS environments for deployments on this team.",
  )
  .configureHelp({ showGlobalOptions: true })
  .allowExcessArguments(false)
  .addDeploymentSelectionOptions(actionDescription("Provision WorkOS team for"))
  .action(async (_options, cmd) => {
    const options = cmd.optsWithGlobals();
    const { ctx, deployment } = await selectEnvDeployment(options);

    // Check if there's already an associated WorkOS team
    const { hasAssociatedWorkosTeam, teamId } =
      await getDeploymentCanProvisionWorkOSEnvironments(
        ctx,
        deployment.deploymentName,
      );

    if (hasAssociatedWorkosTeam) {
      logMessage(
        chalkStderr.yellow(
          "This Convex team already has an associated WorkOS team.",
        ),
      );
      logMessage(
        chalkStderr.dim(
          "Use 'npx convex integration workos status' to view details.",
        ),
      );
      return;
    }

    // Use the shared provisioning flow
    const result = await provisionWorkosTeamInteractive(
      ctx,
      deployment.deploymentName,
      teamId,
      deployment.deploymentType,
    );

    if (!result.success) {
      logMessage(chalkStderr.gray("Cancelled."));
      return;
    }

    // Success!
    logMessage(
      chalkStderr.green(
        `\n✓ Successfully created WorkOS team "${result.workosTeamName}" (${result.workosTeamId})`,
      ),
    );
    logMessage(
      chalkStderr.dim(
        "You can now provision WorkOS environments for deployments on this team.",
      ),
    );
  });

const workosDisconnectTeam = new Command("disconnect-team")
  .summary("Disconnect WorkOS team from Convex team")
  .description(
    "Remove the associated WorkOS team from this Convex team. " +
      "This is a destructive action that will prevent new WorkOS environments from being provisioned. " +
      "Existing environments will continue to work with their current API keys.",
  )
  .configureHelp({ showGlobalOptions: true })
  .allowExcessArguments(false)
  .addDeploymentSelectionOptions(
    actionDescription("Disconnect WorkOS team for"),
  )
  .action(async (_options, cmd) => {
    const options = cmd.optsWithGlobals();
    const { ctx, deployment } = await selectEnvDeployment(options);

    // Check if there's an associated WorkOS team
    const { hasAssociatedWorkosTeam, teamId } =
      await getDeploymentCanProvisionWorkOSEnvironments(
        ctx,
        deployment.deploymentName,
      );

    if (!hasAssociatedWorkosTeam) {
      logMessage(
        chalkStderr.yellow(
          "This Convex team does not have an associated WorkOS team.",
        ),
      );
      return;
    }

    const info = await getTeamAndProjectSlugForDeployment(ctx, {
      deploymentName: deployment.deploymentName,
    });

    logMessage(
      chalkStderr.yellow(
        `Warning: This will disconnect the WorkOS team from Convex team "${info?.teamSlug}".`,
      ),
    );
    logMessage(
      "AuthKit environments provisioned for Convex deployments on this team will no longer use this WorkOS team to provision environments.",
    );
    logMessage(
      chalkStderr.dim(
        "Existing WorkOS environments will continue to work with their current API keys.",
      ),
    );

    const confirmed = await promptYesNo(ctx, {
      message: "Are you sure you want to disconnect this WorkOS team?",
      default: false,
    });

    if (!confirmed) {
      logMessage(chalkStderr.gray("Cancelled."));
      return;
    }

    const result = await disconnectWorkOSTeam(ctx, teamId);

    if (!result.success) {
      if (result.error === "not_associated") {
        logMessage(
          chalkStderr.yellow(
            "This Convex team does not have an associated WorkOS team.",
          ),
        );
        return;
      }
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: `Failed to disconnect WorkOS team: ${result.message}`,
      });
    }

    logFinishedStep(
      `Successfully disconnected WorkOS team "${result.workosTeamName}" (${result.workosTeamId})`,
    );
  });

const workosInvite = new Command("invite")
  .summary("Invite yourself to the WorkOS team")
  .description(
    "Send an invitation to join the WorkOS team associated with your Convex team",
  )
  .option("--email <email>", "Email address to invite (skips validation)")
  .configureHelp({ showGlobalOptions: true })
  .allowExcessArguments(false)
  .addDeploymentSelectionOptions(
    actionDescription("Invite yourself to WorkOS team for"),
  )
  .action(async (options, cmd) => {
    const allOptions = cmd.optsWithGlobals();
    const { ctx, deployment } = await selectEnvDeployment(allOptions);

    // Get team info first
    const info = await fetchTeamAndProject(ctx, deployment.deploymentName);

    let emailToInvite: string;

    // If email was provided as flag, use it directly (skip CLI validation)
    if (options.email) {
      emailToInvite = options.email;
    } else {
      // Get emails eligible for invitation (all verified emails except those that are admin of a different team)
      const { eligibleEmails, adminEmail } = await getInvitationEligibleEmails(
        ctx,
        info.teamId,
      );

      // Combine eligible emails with admin email (admin email is always an option for re-invitation)
      const allInvitableEmails = [...eligibleEmails];
      if (adminEmail && !allInvitableEmails.includes(adminEmail)) {
        allInvitableEmails.push(adminEmail);
      }

      if (allInvitableEmails.length === 0) {
        logMessage(
          "You don't have any verified emails available for invitation.",
        );
        logMessage(
          "This could be because all your verified emails are already admin of other WorkOS teams.",
        );
        return;
      }

      // Let user select which email to use
      emailToInvite = await promptOptions(ctx, {
        message: "Which email would you like to invite to the WorkOS team?",
        choices: allInvitableEmails.map((email) => ({
          name: email + (email === adminEmail ? " (admin email)" : ""),
          value: email,
        })),
        default: allInvitableEmails[0],
      });

      // Confirm before sending
      const confirmed = await promptYesNo(ctx, {
        message: `Send invitation to ${emailToInvite}?`,
        default: true,
      });

      if (!confirmed) {
        logMessage("Invitation cancelled.");
        return;
      }
    }

    logMessage(`Sending invitation to ${emailToInvite}...`);

    const result = await inviteToWorkosTeam(ctx, info.teamId, emailToInvite);

    if (result.result === "success") {
      logMessage(
        `✓ Successfully sent invitation to ${result.email} with role ${result.roleSlug}`,
      );
      logMessage(
        "Check your email for the invitation link to join the WorkOS team.",
      );
    } else if (result.result === "teamNotProvisioned") {
      logMessage(
        `✗ ${result.message}. Run 'npx convex integration workos provision-environment' first.`,
      );
    } else if (result.result === "alreadyInWorkspace") {
      logMessage(
        `✗ ${result.message}. This usually means the email is already used in another WorkOS workspace.`,
      );
    }
  });

// Project environment commands
const workosProjectEnvList = new Command("list-project-environments")
  .summary("List WorkOS environments for current project")
  .description(
    "List all WorkOS AuthKit environments created for the current project.\n" +
      "These environments can be used across multiple deployments.",
  )
  .addDeploymentSelectionOptions(
    actionDescription("List project environments for"),
  )
  .action(async (_options, cmd) => {
    const options = cmd.optsWithGlobals();
    const { ctx, deployment } = await selectEnvDeployment(options);

    const info = await fetchTeamAndProject(ctx, deployment.deploymentName);

    logMessage("Fetching project WorkOS environments...");

    try {
      const environments = await listProjectWorkOSEnvironments(
        ctx,
        info.projectId,
      );

      if (environments.length === 0) {
        logMessage("No WorkOS environments found for this project.");
        logMessage(
          chalkStderr.gray(
            "Create one with: npx convex integration workos create-project-environment --name <name>",
          ),
        );
      } else {
        logMessage(chalkStderr.bold("WorkOS Project Environments:"));
        for (const env of environments) {
          const prodLabel = env.isProduction
            ? chalkStderr.yellow(" (production)")
            : "";
          logMessage(
            `  ${chalkStderr.green(env.userEnvironmentName)}${prodLabel} - Client ID: ${env.workosClientId}`,
          );
        }
      }
    } catch (error) {
      logMessage(
        chalkStderr.red(`Failed to list environments: ${String(error)}`),
      );
    }
  });

const workosProjectEnvCreate = new Command("create-project-environment")
  .summary("Create a new WorkOS environment for the project")
  .description(
    "Create a new WorkOS AuthKit environment for this project.\n" +
      "The environment can be used across multiple deployments.",
  )
  .requiredOption("--name <name>", "Name for the new environment")
  .option("--production", "Mark this environment as a production environment")
  .addDeploymentSelectionOptions(
    actionDescription("Create project environment for"),
  )
  .action(async (_options, cmd) => {
    const options = cmd.optsWithGlobals();
    const environmentName = options.name as string;
    const isProduction = options.production as boolean | undefined;
    const { ctx, deployment } = await selectEnvDeployment(options);

    const info = await fetchTeamAndProject(ctx, deployment.deploymentName);

    showSpinner(
      `Creating project-level WorkOS environment '${environmentName}'...`,
    );

    try {
      const response = await createProjectWorkOSEnvironment(
        ctx,
        info.projectId,
        environmentName,
        isProduction,
      );

      stopSpinner();
      logFinishedStep(`Created WorkOS environment '${environmentName}'`);

      logMessage("");
      logMessage(chalkStderr.bold("Environment Details:"));
      logMessage(`  Name: ${response.userEnvironmentName}`);
      logMessage(`  Client ID: ${response.workosClientId}`);
      logMessage(`  API Key: ${response.workosApiKey}`);
    } catch (error: any) {
      stopSpinner();
      if (error?.message?.includes("NoWorkOSTeam")) {
        logMessage(
          chalkStderr.red(
            "Your team doesn't have a WorkOS integration configured yet.",
          ),
        );
        logMessage(
          "Please run 'npx convex integration workos provision-team' first.",
        );
      } else if (error?.message?.includes("duplicate")) {
        logMessage(
          chalkStderr.red(
            `An environment named '${environmentName}' already exists for this project.`,
          ),
        );
      } else if (error?.message?.includes("TooManyEnvironments")) {
        logMessage(
          chalkStderr.red(
            "You've reached the limit of 10 WorkOS environments per project. If you need more, please contact support.",
          ),
        );
      } else {
        logMessage(chalkStderr.red(`Failed to create environment: ${error}`));
      }
    }
  });

const workosProjectEnvDelete = new Command("delete-project-environment")
  .summary("Delete a WorkOS environment from the project")
  .description(
    "Delete a WorkOS environment from this project.\n" +
      "This will permanently remove the environment and its credentials.\n" +
      "Use the client ID shown in list-project-environments output.",
  )
  .requiredOption(
    "--client-id <clientId>",
    "WorkOS client ID of the environment to delete (shown in list output)",
  )
  .addDeploymentSelectionOptions(
    actionDescription("Delete project environment for"),
  )
  .action(async (_options, cmd) => {
    const options = cmd.optsWithGlobals();
    const clientId = options.clientId as string;
    const { ctx, deployment } = await selectEnvDeployment(options);

    const info = await fetchTeamAndProject(ctx, deployment.deploymentName);

    // Confirm deletion
    const confirmed = await promptYesNo(ctx, {
      message: `Are you sure you want to delete environment with client ID '${clientId}'?`,
      default: false,
    });

    if (!confirmed) {
      logMessage("Deletion cancelled.");
      return;
    }

    showSpinner(
      `Deleting project WorkOS environment (this can take a while)...`,
    );

    try {
      await deleteProjectWorkOSEnvironment(ctx, info.projectId, clientId);
      stopSpinner();
      logFinishedStep(`Deleted environment with client ID '${clientId}'`);
    } catch (error: any) {
      stopSpinner();
      if (error?.message?.includes("not found")) {
        logMessage(
          chalkStderr.red(
            `Environment with client ID '${clientId}' not found.`,
          ),
        );
      } else {
        logMessage(chalkStderr.red(`Failed to delete environment: ${error}`));
      }
    }
  });

const workos = new Command("workos")
  .summary("WorkOS integration commands")
  .description("Manage WorkOS team provisioning and environment setup")
  .addCommand(workosProvisionEnvironment)
  .addCommand(workosTeamStatus)
  .addCommand(workosProvisionTeam)
  .addCommand(workosDisconnectTeam)
  .addCommand(workosInvite)
  .addCommand(workosProjectEnvList)
  .addCommand(workosProjectEnvCreate)
  .addCommand(workosProjectEnvDelete);

export const integration = new Command("integration")
  .summary("Integration commands")
  .description("Commands for managing third-party integrations")
  .addCommand(workos);
