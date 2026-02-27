import { Command } from "@commander-js/extra-typings";
import { chalkStderr } from "chalk";
import { Context, oneoffContext } from "../bundler/context.js";
import {
  DeploymentSelectionOptions,
  deploymentSelectionWithinProjectFromOptions,
  DetailedDeploymentCredentials,
  loadSelectedDeploymentCredentials,
} from "./lib/api.js";
import { actionDescription } from "./lib/command.js";
import { ensureHasConvexDependency } from "./lib/utils/utils.js";
import {
  envGetInDeploymentAction,
  envListInDeployment,
  envRemoveInDeployment,
  envSetInDeployment,
} from "./lib/env.js";
import { getDeploymentSelection } from "./lib/deploymentSelection.js";
import { withRunningBackend } from "./lib/localDeployment/run.js";

const envSet = new Command("set")
  // Pretend value is required
  .usage("[options] <name> <value>")
  .arguments("[name] [value]")
  .summary("Set a variable")
  .description(
    "Set environment variables on your deployment.\n\n" +
      "  npx convex env set NAME 'value'\n" +
      "  npx convex env set NAME # omit a value to set one interactively\n" +
      "  npx convex env set NAME --from-file value.txt\n" +
      "  npx convex env set --from-file .env.defaults\n" +
      "When setting multiple values, it will refuse all changes if any " +
      "variables are already set to different values by default. " +
      "Pass --force to overwrite the provided values.\n",
  )
  .option(
    "--from-file <file>",
    "Read environment variables from a .env file. Without --force, fails if any existing variable has a different value.",
  )
  .option(
    "--force",
    "When setting multiple variables, overwrite existing environment variable values instead of failing on mismatch.",
  )
  .configureHelp({ showGlobalOptions: true })
  .allowExcessArguments(false)
  .action(async (name, value, cmdOptions, cmd) => {
    // Note: We use `as` here because optsWithGlobals() type inference doesn't
    // include global options from the parent command (added via addDeploymentSelectionOptions)
    const options = cmd.optsWithGlobals() as DeploymentSelectionOptions;
    const { ctx, deployment } = await selectEnvDeployment(options);
    await ensureHasConvexDependency(ctx, "env set");
    await withRunningBackend({
      ctx,
      deployment,
      action: async () => {
        const didAnything = await envSetInDeployment(
          ctx,
          deployment,
          name,
          value,
          cmdOptions,
        );
        if (didAnything === false) {
          cmd.outputHelp({ error: true });
          return await ctx.crash({
            exitCode: 1,
            errorType: "fatal",
            printedMessage:
              "error: No environment variables specified to be set.",
          });
        }
      },
    });
  });

async function selectEnvDeployment(
  options: DeploymentSelectionOptions,
): Promise<{
  ctx: Context;
  deployment: {
    deploymentUrl: string;
    adminKey: string;
    deploymentNotice: string;
    deploymentFields: DetailedDeploymentCredentials["deploymentFields"];
  };
}> {
  const ctx = await oneoffContext(options);
  const deploymentSelection = await getDeploymentSelection(ctx, options);
  const selectionWithinProject =
    deploymentSelectionWithinProjectFromOptions(options);
  const {
    adminKey,
    url: deploymentUrl,
    deploymentFields,
  } = await loadSelectedDeploymentCredentials(
    ctx,
    deploymentSelection,
    selectionWithinProject,
    { ensureLocalRunning: false },
  );

  const deploymentNotice =
    deploymentFields !== null
      ? ` (on ${chalkStderr.bold(deploymentFields.deploymentType)} deployment ${chalkStderr.bold(deploymentFields.deploymentName)})`
      : "";
  const result = {
    ctx,
    deployment: {
      deploymentUrl,
      adminKey,
      deploymentNotice,
      deploymentFields,
    },
  };
  return result;
}

const envGet = new Command("get")
  .arguments("<name>")
  .summary("Print a variable's value")
  .description("Print a variable's value: `npx convex env get NAME`")
  .configureHelp({ showGlobalOptions: true })
  .allowExcessArguments(false)
  .action(async (envVarName, _options, cmd) => {
    const options = cmd.optsWithGlobals();
    const { ctx, deployment } = await selectEnvDeployment(options);
    await ensureHasConvexDependency(ctx, "env get");
    await withRunningBackend({
      ctx,
      deployment,
      action: async () => {
        await envGetInDeploymentAction(ctx, deployment, envVarName);
      },
    });
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
    const options = cmd.optsWithGlobals();
    const { ctx, deployment } = await selectEnvDeployment(options);
    await ensureHasConvexDependency(ctx, "env remove");
    await withRunningBackend({
      ctx,
      deployment,
      action: async () => {
        await envRemoveInDeployment(ctx, deployment, name);
      },
    });
  });

const envList = new Command("list")
  .summary("List all variables")
  .description("List all variables: `npx convex env list`")
  .configureHelp({ showGlobalOptions: true })
  .allowExcessArguments(false)
  .action(async (_options, cmd) => {
    const options = cmd.optsWithGlobals();
    const { ctx, deployment } = await selectEnvDeployment(options);
    await ensureHasConvexDependency(ctx, "env list");
    await withRunningBackend({
      ctx,
      deployment,
      action: async () => {
        await envListInDeployment(ctx, deployment);
      },
    });
  });

export const env = new Command("env")
  .summary("Set and view environment variables")
  .description(
    "Set and view environment variables on your deployment\n\n" +
      "  Set a variable: `npx convex env set NAME 'value'`\n" +
      "  Set interactively: `npx convex env set NAME`\n" +
      "  Set multiple from file: `npx convex env set --from-file .env`\n" +
      "  Unset a variable: `npx convex env remove NAME`\n" +
      "  List all variables: `npx convex env list`\n" +
      "  Print a variable's value: `npx convex env get NAME`\n\n" +
      "By default, this sets and views variables on your dev deployment.",
  )
  .addCommand(envSet)
  .addCommand(envGet)
  .addCommand(envRemove)
  .addCommand(envList)
  .helpCommand(false)
  .addDeploymentSelectionOptions(
    actionDescription("Set and view environment variables on"),
  );
