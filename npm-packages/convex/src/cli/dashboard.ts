import { Command } from "@commander-js/extra-typings";
import chalk from "chalk";
import open from "open";
import {
  Context,
  logMessage,
  logOutput,
  logWarning,
  oneoffContext,
} from "../bundler/context.js";
import {
  deploymentSelectionWithinProjectFromOptions,
  loadSelectedDeploymentCredentials,
} from "./lib/api.js";
import { actionDescription } from "./lib/command.js";
import { getDeploymentSelection } from "./lib/deploymentSelection.js";
import { isTryItOutDeployment } from "./lib/deployment.js";
import { loadDashboardConfig } from "./lib/localDeployment/filePaths.js";
import { DEFAULT_LOCAL_DASHBOARD_API_PORT } from "./lib/localDeployment/dashboard.js";
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
    if (isTryItOutDeployment(deployment.deploymentFields.deploymentName)) {
      const dashboardConfig = loadDashboardConfig(ctx);
      const warningMessage = `You are not currently running the dashboard locally. Make sure \`npx convex dev\` is running and try again.`;
      if (dashboardConfig === null) {
        logWarning(ctx, warningMessage);
        return;
      }

      const queryString =
        dashboardConfig.apiPort !== DEFAULT_LOCAL_DASHBOARD_API_PORT
          ? `?apiPort=${dashboardConfig.apiPort}`
          : "";
      const dashboardUrl = `http://127.0.0.1:${dashboardConfig.port}${queryString}`;
      const response = await fetch(dashboardUrl);
      if (!response.ok) {
        logWarning(ctx, warningMessage);
        return;
      }
      await logOrOpenUrl(ctx, dashboardUrl, options.open);
      return;
    }

    const loginUrl = deploymentDashboardUrlPage(
      deployment.deploymentFields.deploymentName,
      "",
    );

    await logOrOpenUrl(ctx, loginUrl, options.open);
  });

async function logOrOpenUrl(ctx: Context, url: string, shouldOpen: boolean) {
  if (shouldOpen) {
    logMessage(ctx, chalk.gray(`Opening ${url} in the default browser...`));
    await open(url);
  } else {
    logOutput(ctx, url);
  }
}

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
