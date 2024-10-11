import { Command } from "@commander-js/extra-typings";
import chalk from "chalk";
import open from "open";
import { logMessage, logOutput, oneoffContext } from "../bundler/context.js";
import {
  deploymentSelectionFromOptions,
  fetchDeploymentCredentialsProvisionProd,
} from "./lib/api.js";
import { actionDescription } from "./lib/command.js";

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
    const ctx = oneoffContext();

    const deploymentSelection = deploymentSelectionFromOptions(options);
    const { deploymentName } = await fetchDeploymentCredentialsProvisionProd(
      ctx,
      deploymentSelection,
    );

    if (deploymentName === undefined) {
      return await ctx.crash({
        exitCode: 1,
        errorType: "invalid filesystem data",
        printedMessage: `No Convex deployment configured, run \`${chalk.bold(
          "npx convex dev",
        )}\``,
      });
    }

    const loginUrl = await deploymentDashboardUrlPage(deploymentName, "");

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
