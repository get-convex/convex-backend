import { Command } from "@commander-js/extra-typings";
import { oneoffContext } from "../bundler/context.js";
import {
  deploymentSelectionFromOptions,
  fetchDeploymentCredentialsProvisionProd,
} from "./lib/api.js";
import { actionDescription } from "./lib/command.js";
import { logsForDeployment } from "./lib/logs.js";

export const logs = new Command("logs")
  .summary("Watch logs from your deployment")
  .description(
    "Stream function logs from your Convex deployment.\nBy default, this streams from your project's dev deployment.",
  )
  .allowExcessArguments(false)
  .addLogsOptions()
  .addDeploymentSelectionOptions(actionDescription("Watch logs from"))
  .showHelpAfterError()
  .action(async (cmdOptions) => {
    const ctx = oneoffContext();

    const deploymentSelection = deploymentSelectionFromOptions(cmdOptions);
    const credentials = await fetchDeploymentCredentialsProvisionProd(
      ctx,
      deploymentSelection,
    );
    const deploymentName = credentials.deploymentName
      ? ` ${credentials.deploymentName}`
      : "";
    const deploymentNotice = ` for ${cmdOptions.prod ? "production" : "dev"} deployment${deploymentName}`;
    await logsForDeployment(ctx, credentials, {
      history: cmdOptions.history,
      success: cmdOptions.success,
      deploymentNotice,
    });
  });
