import { oneoffContext } from "../bundler/context.js";
import {
  deploymentSelectionFromOptions,
  fetchDeploymentCredentialsWithinCurrentProject,
} from "./lib/api.js";
import { Command, Option } from "@commander-js/extra-typings";
import { actionDescription } from "./lib/command.js";
import { functionSpecForDeployment } from "./lib/functionSpec.js";

export const functionSpec = new Command("function-spec")
  .summary("List function metadata from your deployment")
  .description(
    "List argument and return values to your Convex functions.\n\n" +
      "By default, this inspects your dev deployment.",
  )
  .allowExcessArguments(false)
  .addOption(new Option("--file", "Output as JSON to a file."))
  .addDeploymentSelectionOptions(
    actionDescription("Read function metadata from"),
  )
  .showHelpAfterError()
  .action(async (options) => {
    const ctx = oneoffContext();
    const deploymentSelection = await deploymentSelectionFromOptions(
      ctx,
      options,
    );

    const { adminKey, url: deploymentUrl } =
      await fetchDeploymentCredentialsWithinCurrentProject(
        ctx,
        deploymentSelection,
      );

    await functionSpecForDeployment(ctx, {
      deploymentUrl,
      adminKey,
      file: !!options.file,
    });
  });
