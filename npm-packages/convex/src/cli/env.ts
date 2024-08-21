import { Command } from "@commander-js/extra-typings";
import chalk from "chalk";
import {
  Context,
  logFailure,
  logFinishedStep,
  logMessage,
  logOutput,
  oneoffContext,
} from "../bundler/context.js";
import {
  DeploymentSelectionOptions,
  deploymentSelectionFromOptions,
  fetchDeploymentCredentialsWithinCurrentProject,
} from "./lib/api.js";
import { actionDescription } from "./lib/command.js";
import { runQuery } from "./lib/run.js";
import {
  deploymentFetch,
  ensureHasConvexDependency,
  logAndHandleFetchError,
} from "./lib/utils/utils.js";

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
    const ctx = oneoffContext;
    await ensureHasConvexDependency(ctx, "env set");
    const [name, value] = await allowEqualsSyntax(
      ctx,
      originalName,
      originalValue,
    );
    const where = await callUpdateEnvironmentVariables(ctx, options, [
      { name, value },
    ]);
    const formatted = /\s/.test(value) ? `"${value}"` : value;
    logFinishedStep(
      ctx,
      `Successfully set ${chalk.bold(name)} to ${chalk.bold(formatted)}${where}`,
    );
  });

async function allowEqualsSyntax(
  ctx: Context,
  name: string,
  value: string | undefined,
) {
  if (value === undefined) {
    if (/^[a-zA-Z][a-zA-Z0-9_]+=/.test(name)) {
      return name.split("=", 2);
    } else {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: "error: missing required argument 'value'",
      });
    }
  }
  return [name, value];
}

const envGet = new Command("get")
  .arguments("<name>")
  .summary("Print a variable's value")
  .description("Print a variable's value: `npx convex env get NAME`")
  .configureHelp({ showGlobalOptions: true })
  .allowExcessArguments(false)
  .action(async (envVarName, _options, cmd) => {
    const ctx = oneoffContext;
    await ensureHasConvexDependency(ctx, "env get");
    const options = cmd.optsWithGlobals();
    const deploymentSelection = deploymentSelectionFromOptions(options);
    const { adminKey, url } =
      await fetchDeploymentCredentialsWithinCurrentProject(
        ctx,
        deploymentSelection,
      );

    const envVar = (await runQuery(
      ctx,
      url,
      adminKey,
      "_system/cli/queryEnvironmentVariables:get",
      { name: envVarName },
    )) as EnvVar | null;
    if (envVar === null) {
      logFailure(ctx, `Environment variable "${envVarName}" not found.`);
      return;
    }
    const { value } = envVar;
    logOutput(ctx, `${value}`);
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
    const ctx = oneoffContext;
    const options = cmd.optsWithGlobals();
    await ensureHasConvexDependency(ctx, "env remove");
    const where = await callUpdateEnvironmentVariables(ctx, options, [
      { name },
    ]);
    logFinishedStep(ctx, `Successfully unset ${chalk.bold(name)}${where}`);
  });

const envList = new Command("list")
  .summary("List all variables")
  .description("List all variables: `npx convex env list`")
  .configureHelp({ showGlobalOptions: true })
  .allowExcessArguments(false)
  .action(async (_options, cmd) => {
    const ctx = oneoffContext;
    await ensureHasConvexDependency(ctx, "env list");
    const options = cmd.optsWithGlobals();
    const deploymentSelection = deploymentSelectionFromOptions(options);
    const { adminKey, url } =
      await fetchDeploymentCredentialsWithinCurrentProject(
        ctx,
        deploymentSelection,
      );

    const envs = (await runQuery(
      ctx,
      url,
      adminKey,
      "_system/cli/queryEnvironmentVariables",
      {},
    )) as EnvVar[];
    if (envs.length === 0) {
      logMessage(ctx, "No environment variables set.");
      return;
    }
    for (const { name, value } of envs) {
      logOutput(ctx, `${name}=${value}`);
    }
  });

type EnvVarChange = {
  name: string;
  value?: string;
};

type EnvVar = {
  name: string;
  value: string;
};

async function callUpdateEnvironmentVariables(
  ctx: Context,
  options: DeploymentSelectionOptions,
  changes: EnvVarChange[],
) {
  const deploymentSelection = deploymentSelectionFromOptions(options);
  const { adminKey, url, deploymentName, deploymentType } =
    await fetchDeploymentCredentialsWithinCurrentProject(
      ctx,
      deploymentSelection,
    );
  const fetch = deploymentFetch(url, adminKey);
  try {
    await fetch("/api/update_environment_variables", {
      body: JSON.stringify({ changes }),
      method: "POST",
    });
    return deploymentType !== undefined || deploymentName !== undefined
      ? ` (on${
          deploymentType !== undefined ? " " + chalk.bold(deploymentType) : ""
        } deployment${
          deploymentName !== undefined ? " " + chalk.bold(deploymentName) : ""
        })`
      : "";
  } catch (e) {
    return await logAndHandleFetchError(ctx, e);
  }
}

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
