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
  loadSelectedDeploymentCredentials,
} from "./lib/api.js";
import { actionDescription } from "./lib/command.js";
import { ensureHasConvexDependency } from "./lib/utils/utils.js";
import { getDeploymentSelection } from "./lib/deploymentSelection.js";
import { ensureWorkosEnvironmentProvisioned } from "./lib/workos/workos.js";
import {
  getCandidateEmailsForWorkIntegration,
  getWorkosEnvironmentHealth,
  getWorkosTeamHealth,
} from "./lib/workos/platformApi.js";
import { logMessage } from "../bundler/log.js";

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
    } else if (teamHealth.teamStatus === "Inactive") {
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
  .action(async (_options, cmd) => {
    const options = cmd.optsWithGlobals();
    const { ctx, deployment } = await selectEnvDeployment(options);
    await ensureHasConvexDependency(
      ctx,
      "integration workos provision-environment",
    );

    try {
      await ensureWorkosEnvironmentProvisioned(
        ctx,
        deployment.deploymentName,
        deployment,
        {
          offerToAssociateWorkOSTeam: true,
          autoProvisionIfWorkOSTeamAssociated: true,
          autoConfigureAuthkitConfig: true,
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
const workos = new Command("workos")
  .summary("WorkOS integration commands")
  .description("Manage WorkOS team provisioning and environment setup")
  .addCommand(workosProvisionEnvironment)
  .addCommand(workosTeamStatus);

export const integration = new Command("integration")
  .summary("Integration commands")
  .description("Commands for managing third-party integrations")
  .addCommand(workos);
