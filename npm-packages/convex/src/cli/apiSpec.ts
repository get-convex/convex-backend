import { logOutput, oneoffContext } from "../bundler/context.js";
import {
  deploymentSelectionFromOptions,
  fetchDeploymentCredentialsWithinCurrentProject,
} from "./lib/api.js";
import { runQuery } from "./lib/run.js";
import { Command } from "@commander-js/extra-typings";
import { actionDescription } from "./lib/command.js";

export const apiSpec = new Command("api-spec")
  .summary("List function metadata from your deployment")
  .description(
    "List argument and return values to your Convex functions.\n\n" +
      "By default, this inspects your dev deployment.",
  )
  .addDeploymentSelectionOptions(
    actionDescription("Read function metadata from"),
  )
  .showHelpAfterError()
  .action(async (options) => {
    const ctx = oneoffContext;
    const deploymentSelection = deploymentSelectionFromOptions(options);

    const { adminKey, url: deploymentUrl } =
      await fetchDeploymentCredentialsWithinCurrentProject(
        ctx,
        deploymentSelection,
      );

    const functions = (await runQuery(
      ctx,
      deploymentUrl,
      adminKey,
      "_system/cli/modules:apiSpec",
      {},
    )) as any[];

    logOutput(ctx, JSON.stringify(functions, null, 2));
  });
