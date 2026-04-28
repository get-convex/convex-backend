import { Command } from "@commander-js/extra-typings";
import { chalkStderr } from "chalk";
import { oneoffContext } from "../bundler/context.js";
import { logFinishedStep, showSpinner } from "../bundler/log.js";
import { loadSelectedDeploymentCredentials } from "./lib/api.js";
import { actionDescription } from "./lib/command.js";
import { getDeploymentSelection } from "./lib/deploymentSelection.js";
import {
  CONVEX_DEPLOY_KEY_ENV_VAR_NAME,
  typedPlatformClient,
} from "./lib/utils/utils.js";

export const deploymentTokenDelete = new Command("delete")
  .summary("Delete an access token")
  .description(
    "Delete an access token. Currently only deploy keys (deployment-scoped access tokens) are supported.\n\n" +
      "The positional `<nameOrToken>` can be the unique name of the deploy key (as passed to `token create`) or the deploy key value itself. The target deployment defaults to the currently-selected one; pass `--deployment` to target a different deployment.\n\n" +
      "  Delete by name:  `npx convex deployment token delete my-token`\n" +
      "  Delete by value: `npx convex deployment token delete 'dev:happy-animal-123|ey...'`\n" +
      "  Target prod:     `npx convex deployment token delete ci-token --deployment prod`",
  )
  .argument(
    "<nameOrToken>",
    "The unique name of the deploy key, or the deploy key value itself.",
  )
  .allowExcessArguments(false)
  .addDeploymentSelectionOptions(actionDescription("Delete a deploy key for"))
  .showHelpAfterError()
  .action(async (nameOrToken, options) => {
    const ctx = await oneoffContext(options);

    const auth = ctx.bigBrainAuth();
    if (auth === null || auth.kind !== "accessToken") {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: `Deleting a deploy key requires being logged in with a personal access token. ${auth === null ? "Run " : `Unset ${CONVEX_DEPLOY_KEY_ENV_VAR_NAME} and run `}${chalkStderr.bold("npx convex login")} and try again.`,
      });
    }

    const deploymentSelection = await getDeploymentSelection(ctx, options);
    const deployment = await loadSelectedDeploymentCredentials(
      ctx,
      deploymentSelection,
      { ensureLocalRunning: false },
    );

    if (deployment.deploymentFields === null) {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage:
          "Cannot delete a deploy key for a self-hosted deployment.",
      });
    }

    const { deploymentName, deploymentType } = deployment.deploymentFields;
    if (deploymentType === "local" || deploymentType === "anonymous") {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: `Cannot delete a deploy key for a ${deploymentType} deployment.`,
      });
    }

    showSpinner(`Deleting deploy key for ${deploymentName}...`);
    await typedPlatformClient(ctx).POST(
      "/deployments/{deployment_name}/delete_deploy_key",
      {
        params: { path: { deployment_name: deploymentName } },
        body: { id: nameOrToken },
      },
    );

    logFinishedStep(
      `Deleted deploy key "${nameOrToken}" for ${deploymentName}.`,
    );
  });
