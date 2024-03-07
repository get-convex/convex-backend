import chalk from "chalk";
import open from "open";
import {
  Context,
  logFailure,
  logMessage,
  oneoffContext,
} from "../bundler/context.js";
import {
  deploymentSelectionFromOptions,
  fetchDeploymentCredentialsProvisionProd,
  fetchTeamAndProject,
} from "./lib/api.js";
import { Command } from "@commander-js/extra-typings";
import { actionDescription } from "./lib/command.js";

const DASHBOARD_HOST = process.env.CONVEX_PROVISION_HOST
  ? "http://localhost:6789"
  : "https://dashboard.convex.dev";

export const dashboard = new Command("dashboard")
  .description("Open the dashboard in the browser")
  .option(
    "--no-open",
    "Don't automatically open the dashboard in the default browser",
  )
  .addDeploymentSelectionOptions(actionDescription("Open the dashboard for"))
  .showHelpAfterError()
  .action(async (options) => {
    const ctx = oneoffContext;

    const deploymentSelection = deploymentSelectionFromOptions(options);
    const { deploymentName } = await fetchDeploymentCredentialsProvisionProd(
      ctx,
      deploymentSelection,
    );

    if (deploymentName === undefined) {
      logFailure(
        ctx,
        "No deployment name, run `npx convex dev` to configure a Convex project",
      );
      return await ctx.crash(1, "invalid filesystem data");
    }

    const loginUrl = await deploymentDashboardUrlPage(ctx, deploymentName, "");

    if (options.open) {
      logMessage(
        ctx,
        chalk.gray(`Opening ${loginUrl} in the default browser...`),
      );
      await open(loginUrl);
    } else {
      console.log(loginUrl);
    }
  });

export async function deploymentDashboardUrlPage(
  ctx: Context,
  configuredDeployment: string | null,
  page: string,
): Promise<string> {
  if (configuredDeployment !== null) {
    const { team, project } = await fetchTeamAndProject(
      ctx,
      configuredDeployment,
    );
    return deploymentDashboardUrl(team, project, configuredDeployment) + page;
  } else {
    // If there is no configured deployment, go to the most recently opened deployment.
    return `${DASHBOARD_HOST}/deployment/${page}`;
  }
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
