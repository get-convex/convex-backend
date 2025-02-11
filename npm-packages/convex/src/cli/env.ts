import { Command } from "@commander-js/extra-typings";
import chalk from "chalk";
import { Context, oneoffContext } from "../bundler/context.js";
import {
  DeploymentSelectionOptions,
  deploymentSelectionFromOptions,
  fetchDeploymentCredentialsWithinCurrentProject,
} from "./lib/api.js";
import { actionDescription } from "./lib/command.js";
import { ensureHasConvexDependency } from "./lib/utils/utils.js";
import {
  envGetInDeployment,
  envListInDeployment,
  envRemoveInDeployment,
  envSetInDeployment,
} from "./lib/env.js";

const envSet = new Command("set")
  // Pretend value is required
  .usage("[options] <name> <value>")
  .arguments("<name> [value]")
  .summary("Set a variable")
  .description(
    "Set a variable: `npx convex env set NAME value`\n" +
      "If the variable already exists, its value is updated.\n\n" +
      "A single `NAME=value` argument is also supported.",
  )
  .configureHelp({ showGlobalOptions: true })
  .allowExcessArguments(false)
  .action(async (originalName, originalValue, _options, cmd) => {
    const options = cmd.optsWithGlobals();
    const ctx = oneoffContext();
    await ensureHasConvexDependency(ctx, "env set");
    const deployment = await selectEnvDeployment(ctx, options);
    await envSetInDeployment(ctx, deployment, originalName, originalValue);
  });

async function selectEnvDeployment(
  ctx: Context,
  options: DeploymentSelectionOptions,
) {
  const deploymentSelection = await deploymentSelectionFromOptions(
    ctx,
    options,
  );
  const { adminKey, url, deploymentName, deploymentType } =
    await fetchDeploymentCredentialsWithinCurrentProject(
      ctx,
      deploymentSelection,
    );
  const deploymentNotice =
    deploymentType !== undefined || deploymentName !== undefined
      ? ` (on${
          deploymentType !== undefined ? " " + chalk.bold(deploymentType) : ""
        } deployment${
          deploymentName !== undefined ? " " + chalk.bold(deploymentName) : ""
        })`
      : "";
  return {
    deploymentUrl: url,
    adminKey,
    deploymentNotice,
  };
}

const envGet = new Command("get")
  .arguments("<name>")
  .summary("Print a variable's value")
  .description("Print a variable's value: `npx convex env get NAME`")
  .configureHelp({ showGlobalOptions: true })
  .allowExcessArguments(false)
  .action(async (envVarName, _options, cmd) => {
    const ctx = oneoffContext();
    await ensureHasConvexDependency(ctx, "env get");
    const options = cmd.optsWithGlobals();
    const deployment = await selectEnvDeployment(ctx, options);
    await envGetInDeployment(ctx, deployment, envVarName);
  });

const envRemove = new Command("remove")
  .alias("rm")
  .alias("unset")
  .arguments("<name>")
  .summary("Unset a variable")
  .description(
    "Unset a variable: `npx convex env remove NAME`\n" +
      "If the variable doesn't exist, the command doesn't do anything and succeeds.",
  )
  .configureHelp({ showGlobalOptions: true })
  .allowExcessArguments(false)
  .action(async (name, _options, cmd) => {
    const ctx = oneoffContext();
    const options = cmd.optsWithGlobals();
    await ensureHasConvexDependency(ctx, "env remove");
    const deployment = await selectEnvDeployment(ctx, options);
    await envRemoveInDeployment(ctx, deployment, name);
  });

const envList = new Command("list")
  .summary("List all variables")
  .description("List all variables: `npx convex env list`")
  .configureHelp({ showGlobalOptions: true })
  .allowExcessArguments(false)
  .action(async (_options, cmd) => {
    const ctx = oneoffContext();
    await ensureHasConvexDependency(ctx, "env list");
    const options = cmd.optsWithGlobals();
    const deployment = await selectEnvDeployment(ctx, options);
    await envListInDeployment(ctx, deployment);
  });

export const env = new Command("env")
  .summary("Set and view environment variables")
  .description(
    "Set and view environment variables on your deployment\n\n" +
      "  Set a variable: `npx convex env set NAME value`\n" +
      "  Unset a variable: `npx convex env remove NAME`\n" +
      "  List all variables: `npx convex env list`\n" +
      "  Print a variable's value: `npx convex env get NAME`\n\n" +
      "By default, this sets and views variables on your dev deployment.",
  )
  .addCommand(envSet)
  .addCommand(envGet)
  .addCommand(envRemove)
  .addCommand(envList)
  .addHelpCommand(false)
  .addDeploymentSelectionOptions(
    actionDescription("Set and view environment variables on"),
  );
