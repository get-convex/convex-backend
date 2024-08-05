import chalk from "chalk";
import { logOutput, oneoffContext } from "../bundler/context.js";
import {
  deploymentSelectionFromOptions,
  fetchDeploymentCredentialsWithinCurrentProject,
} from "./lib/api.js";
import { runQuery } from "./lib/run.js";
import { Command, Option } from "@commander-js/extra-typings";
import { actionDescription } from "./lib/command.js";

export const functionSpec = new Command("function-spec")
  .summary("List function metadata from your deployment")
  .description(
    "List argument and return values to your Convex functions.\n\n" +
      "By default, this inspects your dev deployment.",
  )
  .addOption(new Option("--path", "Output as JSON to this file path."))
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

    const output = JSON.stringify(functions, null, 2);

    if (options.path) {
      const fileName = `function_spec_${Date.now().valueOf()}.json`;
      ctx.fs.writeUtf8File(fileName, output);
      logOutput(ctx, chalk.green(`Wrote function spec to ${fileName}`));
    } else {
      logOutput(ctx, output);
    }
  });
