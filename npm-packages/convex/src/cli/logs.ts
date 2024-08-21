import { Command } from "@commander-js/extra-typings";
import chalk from "chalk";
import { logMessage, oneoffContext } from "../bundler/context.js";
import {
  deploymentSelectionFromOptions,
  fetchDeploymentCredentialsProvisionProd,
} from "./lib/api.js";
import { actionDescription } from "./lib/command.js";
import { watchLogs } from "./lib/logs.js";
import { parseInteger } from "./lib/utils/utils.js";

export const logs = new Command("logs")
  .summary("Watch logs from your deployment")
  .description(
    "Stream function logs from your Convex deployment.\nBy default, this streams from your project's dev deployment.",
  )
  .option(
    "--history [n]",
    "Show `n` most recent logs. Defaults to showing all available logs.",
    parseInteger,
  )
  .option(
    "--success",
    "Print a log line for every successful function execution",
    false,
  )
  .addDeploymentSelectionOptions(actionDescription("Watch logs from"))
  .showHelpAfterError()
  .action(async (cmdOptions) => {
    const ctx = oneoffContext;

    const deploymentSelection = deploymentSelectionFromOptions(cmdOptions);
    const credentials = await fetchDeploymentCredentialsProvisionProd(
      ctx,
      deploymentSelection,
    );
    if (cmdOptions.prod) {
      logMessage(
        ctx,
        chalk.yellow(
          `Watching logs for production deployment ${
            credentials.deploymentName || ""
          }...`,
        ),
      );
    } else {
      logMessage(
        ctx,
        chalk.yellow(
          `Watching logs for dev deployment ${
            credentials.deploymentName || ""
          }...`,
        ),
      );
    }
    await watchLogs(ctx, credentials.url, credentials.adminKey, "stdout", {
      history: cmdOptions.history,
      success: cmdOptions.success,
    });
  });
