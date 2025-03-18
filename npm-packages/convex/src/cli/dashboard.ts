import { Command } from "@commander-js/extra-typings";
import chalk from "chalk";
import open from "open";
import { logMessage, logOutput, oneoffContext } from "../bundler/context.js";
import {
  deploymentSelectionWithinProjectFromOptions,
  loadSelectedDeploymentCredentials,
} from "./lib/api.js";
import { actionDescription } from "./lib/command.js";
import { getDeploymentSelection } from "./lib/deploymentSelection.js";

const DASHBOARD_HOST = process.env.CONVEX_PROVISION_HOST
  ? "http://localhost:6789"
  : "https://dashboard.convex.dev";

export const dashboard = new Command("dashboard")
  .description("Open the dashboard in the browser")
  .allowExcessArguments(false)
  .option(
    "--no-open",
    "Don't automatically open the dashboard in the default browser",
  )
  .addDeploymentSelectionOptions(actionDescription("Open the dashboard for"))
  .showHelpAfterError()
  .action(async (options) => {
    const ctx = await oneoffContext(options);

    const selectionWithinProject =
      await deploymentSelectionWithinProjectFromOptions(ctx, options);
    const deploymentSelection = await getDeploymentSelection(ctx, options);
    const deployment = await loadSelectedDeploymentCredentials(
      ctx,
      deploymentSelection,
      selectionWithinProject,
      { ensureLocalRunning: false },
    );

    if (deployment.deploymentFields === null) {
      const msg = `Self-hosted deployment configured.\n\`${chalk.bold("npx convex dashboard")}\` is not supported for self-hosted deployments.\nSee self-hosting instructions for how to self-host the dashboard.`;
      logMessage(ctx, chalk.yellow(msg));
      return;
    }

    const loginUrl = deploymentDashboardUrlPage(
      deployment.deploymentFields.deploymentName,
      "",
    );

    if (options.open) {
      logMessage(
        ctx,
        chalk.gray(`Opening ${loginUrl} in the default browser...`),
      );
      await open(loginUrl);
    } else {
      logOutput(ctx, loginUrl);
    }
  });

export function deploymentDashboardUrlPage(
  configuredDeployment: string | null,
  page: string,
): string {
  return `${DASHBOARD_HOST}/d/${configuredDeployment}${page}`;
}

export function deploymentDashboardUrl(
  team: string,
  project: string,
  deploymentName: string,
) {
  return `${projectDashboardUrl(team, project)}/${deploymentName}`;
}

export function projectDashboardUrl(team: string, project: string) {
  return `${teamDashboardUrl(team)}/${project}`;
}

export function teamDashboardUrl(team: string) {
  return `${DASHBOARD_HOST}/t/${team}`;
}
