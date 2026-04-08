import { Command } from "@commander-js/extra-typings";
import { Context, oneoffContext } from "../bundler/context.js";
import { loadSelectedDeploymentCredentials } from "./lib/api.js";
import {
  DeploymentSelection,
  getDeploymentSelection,
  deploymentNameFromSelection,
} from "./lib/deploymentSelection.js";
import { parseDeploymentSelector } from "./lib/deploymentSelector.js";
import { updateEnvAndConfigForDeploymentSelection } from "./configure.js";
import { fetchDeploymentCanonicalUrls } from "./lib/deploy2.js";
import { chalkStderr } from "chalk";

export const deploymentSelect = new Command("select")
  .summary("Select the deployment to use when running commands")
  .description(
    "Select the deployment to use when running commands.\n\n" +
      "The deployment will be used by all `npx convex` commands, except `npx convex deploy`. You can also run individual commands on another deployment by using the --deployment flag on that command.\n\n" +
      "Examples:\n" +
      "  npx convex select dev                              # Select your personal cloud dev deployment in the current project\n" +
      "  npx convex select local                            # Select your local deployment\n" +
      "  npx convex select dev/james                        # Select a deployment in the same project by its reference\n" +
      "  npx convex select some-project:dev/james           # Select a deployment in another project in the same team\n" +
      "  npx convex select some-team:some-project:dev/james # Select a deployment in a particular team/project\n",
  )
  .argument("<deployment>", "The deployment to use")
  .allowExcessArguments(false)
  .action(async (selector) => {
    const ctx = await oneoffContext({
      url: undefined,
      adminKey: undefined,
      envFile: undefined,
    });

    // Get the current deployment selection (no flags, just env/config state)
    const currentSelection = await getDeploymentSelection(ctx, {});

    // If no project is configured and the selector needs project context, show a specific error
    const parsed = parseDeploymentSelector(selector);
    if (
      currentSelection.kind === "chooseProject" &&
      parsed.kind !== "inTeamProject" &&
      parsed.kind !== "deploymentName" &&
      parsed.kind !== "local"
    ) {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: `No project configured. Run \`npx convex dev\` to set up a project first, or use a full selector like 'my-team:my-project:dev/james' or 'happy-capybara-123'.`,
      });
    }

    // Resolve the new deployment using the selector relative to the current project
    const newSelection = await getDeploymentSelection(ctx, {
      url: undefined,
      adminKey: undefined,
      envFile: undefined,
      deployment: selector,
    });

    await saveSelectedDeployment(
      ctx,
      selector,
      newSelection,
      deploymentNameFromSelection(currentSelection),
    );
  });

export async function saveSelectedDeployment(
  ctx: Context,
  selector: string,
  selection: DeploymentSelection,
  previousDeploymentName: string | null,
): Promise<void> {
  const deployment = await loadSelectedDeploymentCredentials(ctx, selection, {
    ensureLocalRunning: false,
  });

  if (deployment.deploymentFields === null) {
    // Should be unreachable since for now, `select` only allows users
    // to select deployments that exist in Big Brain
    return ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: null,
      errForSentry: `Unexpected selection in select: ${JSON.stringify(deployment)}`,
    });
  }

  if (deployment.deploymentFields.deploymentType === "prod") {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `Selecting a production deployment is unsupported. To run commands on a production deployment, pass the ${chalkStderr.bold(`--deployment ${selector}`)} flag to each command.`,
    });
  }

  const { convexSiteUrl: siteUrl } =
    deployment.deploymentFields.deploymentType === "local"
      ? { convexSiteUrl: null }
      : await fetchDeploymentCanonicalUrls(ctx, {
          adminKey: deployment.adminKey,
          deploymentUrl: deployment.url,
        });

  await updateEnvAndConfigForDeploymentSelection(
    ctx,
    {
      url: deployment.url,
      siteUrl,
      deploymentName: deployment.deploymentFields.deploymentName,
      teamSlug: deployment.deploymentFields.teamSlug,
      projectSlug: deployment.deploymentFields.projectSlug,
      deploymentType: deployment.deploymentFields.deploymentType,
    },
    previousDeploymentName,
  );
}
