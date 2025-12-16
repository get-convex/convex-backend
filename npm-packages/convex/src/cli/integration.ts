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
} from "./lib/workos/platformApi.js";
import { logFinishedStep, logMessage } from "../bundler/log.js";
import { promptOptions, promptYesNo } from "./lib/utils/prompts.js";

async function selectEnvDeployment(
  options: DeploymentSelectionOptions,
): Promise<{
  ctx: Context;
  deployment: {
    deploymentUrl: string;
    deploymentName: string;
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
  const deploymentNotice =
    deploymentFields !== null
      ? ` (on ${chalkStderr.bold(deploymentFields.deploymentType)} deployment ${chalkStderr.bold(deploymentFields.deploymentName)})`
      : "";
  return {
    ctx,
    deployment: {
      deploymentName: deploymentFields!.deploymentName,
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
    } else if (teamHealth.teamStatus === "inactive") {
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

    const environmentName = options.name as string | undefined;

    try {
      await ensureWorkosEnvironmentProvisioned(
        ctx,
        deployment.deploymentName,
        deployment,
        {
          offerToAssociateWorkOSTeam: true,
          autoProvisionIfWorkOSTeamAssociated: true,
          autoConfigureAuthkitConfig: true,
          ...(environmentName !== undefined && { environmentName }),
        },
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
  .configureHelp({ showGlobalOptions: true })
  .allowExcessArguments(false)
  .addDeploymentSelectionOptions(
    actionDescription("Invite yourself to WorkOS team for"),
  )
  .action(async (_options, cmd) => {
    const options = cmd.optsWithGlobals();
    const { ctx, deployment } = await selectEnvDeployment(options);

    // Get team info first
    const info = await fetchTeamAndProject(ctx, deployment.deploymentName);

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
    const emailToInvite = await promptOptions(ctx, {
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

const workos = new Command("workos")
  .summary("WorkOS integration commands")
  .description("Manage WorkOS team provisioning and environment setup")
  .addCommand(workosProvisionEnvironment)
  .addCommand(workosTeamStatus)
  .addCommand(workosProvisionTeam)
  .addCommand(workosDisconnectTeam)
  .addCommand(workosInvite);

export const integration = new Command("integration")
  .summary("Integration commands")
  .description("Commands for managing third-party integrations")
  .addCommand(workos);
