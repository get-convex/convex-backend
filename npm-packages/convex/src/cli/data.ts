import chalk from "chalk";
import { oneoffContext } from "../bundler/context.js";
import {
  deploymentSelectionFromOptions,
  fetchDeploymentCredentialsProvisionProd,
} from "./lib/api.js";
import { Command } from "@commander-js/extra-typings";
import { actionDescription } from "./lib/command.js";
import { dataInDeployment } from "./lib/data.js";

export const data = new Command("data")
  .summary("List tables and print data from your database")
  .description(
    "Inspect your Convex deployment's database.\n\n" +
      "  List tables: `npx convex data`\n" +
      "  List documents in a table: `npx convex data tableName`\n\n" +
      "By default, this inspects your dev deployment.",
  )
  .allowExcessArguments(false)
  .addDataOptions()
  .addDeploymentSelectionOptions(actionDescription("Inspect the database in"))
  .showHelpAfterError()
  .action(async (tableName, options) => {
    const ctx = oneoffContext();
    const deploymentSelection = deploymentSelectionFromOptions(options);

    const {
      adminKey,
      url: deploymentUrl,
      deploymentName,
    } = await fetchDeploymentCredentialsProvisionProd(ctx, deploymentSelection);

    const deploymentNotice = deploymentName
      ? `${chalk.bold(deploymentName)} deployment's `
      : "";

    await dataInDeployment(ctx, {
      deploymentUrl,
      adminKey,
      deploymentNotice,
      tableName,
      ...options,
    });
  });
