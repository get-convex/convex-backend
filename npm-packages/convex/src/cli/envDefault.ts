import { Command } from "@commander-js/extra-typings";
import {
  envGet,
  envList,
  envRemove,
  envSet,
  EnvVarBackend,
} from "./lib/env.js";
import { ensureHasConvexDependency } from "./lib/utils/utils.js";
import { defaultEnvBackend } from "./lib/defaultEnv.js";
import {
  CloudDeploymentType,
  DeploymentSelectionOptions,
  DeploymentType,
  fetchTeamAndProject,
} from "./lib/api.js";
import { Context } from "../bundler/context.js";
import { selectEnvDeployment } from "./env.js";

const envDefaultSet = new Command("set")
  .usage("[options] <name> <value>")
  .arguments("[name] [value]")
  .summary("Set a default variable")
  .description(
    "Set default environment variables for your project's deployment type.\n\n" +
      "  npx convex env default set NAME 'value'\n" +
      "  npx convex env default set NAME # omit a value to set one interactively\n" +
      "  npx convex env default set NAME --from-file value.txt\n" +
      "  npx convex env default set --from-file .env.defaults\n" +
      "When setting multiple values, it will refuse all changes if any " +
      "variables are already set to different values by default. " +
      "Pass --force to overwrite the provided values.\n" +
      "The deployment type is determined by the current deployment (local maps to dev).\n",
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
    const options = cmd.optsWithGlobals() as DeploymentSelectionOptions;
    const { ctx, deployment } = await selectEnvDeployment(options);
    await ensureHasConvexDependency(ctx, "env default set");
    const backend = await resolveDefaultEnvBackend(
      ctx,
      deployment.deploymentFields,
    );
    const didAnything = await envSet(ctx, backend, name, value, cmdOptions);
    if (didAnything === false) {
      cmd.outputHelp({ error: true });
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: "error: No environment variables specified to be set.",
      });
    }
  });

const envDefaultGet = new Command("get")
  .arguments("<name>")
  .summary("Print a default variable's value")
  .description(
    "Print a default variable's value: `npx convex env default get NAME`\n" +
      "The deployment type is determined by the current deployment (local maps to dev).",
  )
  .configureHelp({ showGlobalOptions: true })
  .allowExcessArguments(false)
  .action(async (envVarName, _options, cmd) => {
    const options = cmd.optsWithGlobals() as DeploymentSelectionOptions;
    const { ctx, deployment } = await selectEnvDeployment(options);
    await ensureHasConvexDependency(ctx, "env default get");
    const backend = await resolveDefaultEnvBackend(
      ctx,
      deployment.deploymentFields,
    );
    await envGet(ctx, backend, envVarName);
  });

const envDefaultRemove = new Command("remove")
  .alias("rm")
  .alias("unset")
  .arguments("<name>")
  .summary("Unset a default variable")
  .description(
    "Unset a default variable: `npx convex env default remove NAME`\n" +
      "If the variable doesn't exist, the command doesn't do anything and succeeds.\n" +
      "The deployment type is determined by the current deployment (local maps to dev).",
  )
  .configureHelp({ showGlobalOptions: true })
  .allowExcessArguments(false)
  .action(async (name, _options, cmd) => {
    const options = cmd.optsWithGlobals() as DeploymentSelectionOptions;
    const { ctx, deployment } = await selectEnvDeployment(options);
    await ensureHasConvexDependency(ctx, "env default remove");
    const backend = await resolveDefaultEnvBackend(
      ctx,
      deployment.deploymentFields,
    );
    await envRemove(ctx, backend, name);
  });

const envDefaultList = new Command("list")
  .summary("List all default variables")
  .description(
    "List all default variables: `npx convex env default list`\n" +
      "The deployment type is determined by the current deployment (local maps to dev).",
  )
  .configureHelp({ showGlobalOptions: true })
  .allowExcessArguments(false)
  .action(async (_options, cmd) => {
    const options = cmd.optsWithGlobals() as DeploymentSelectionOptions;
    const { ctx, deployment } = await selectEnvDeployment(options);
    await ensureHasConvexDependency(ctx, "env default list");
    const backend = await resolveDefaultEnvBackend(
      ctx,
      deployment.deploymentFields,
    );
    await envList(ctx, backend);
  });

export const envDefault = new Command("default")
  .summary("Manage project-level default environment variables")
  .description(
    "Manage default environment variables for your project.\n\n" +
      "The default environment variables read and written to by this command are the ones for the deployment type of the current deployment (i.e. dev in most cases).\n\n" +
      "  Set a default variable: `npx convex env default set NAME 'value'`\n" +
      "  Unset a default variable: `npx convex env default remove NAME`\n" +
      "  List all default variables: `npx convex env default list`\n" +
      "  Print a default variable's value: `npx convex env default get NAME`\n\n",
  )
  .addCommand(envDefaultSet)
  .addCommand(envDefaultGet)
  .addCommand(envDefaultRemove)
  .addCommand(envDefaultList)
  .helpCommand(false);

export async function resolveDefaultEnvBackend(
  ctx: Context,
  deploymentFields: {
    deploymentName: string;
    deploymentType: DeploymentType;
  } | null,
): Promise<EnvVarBackend> {
  if (deploymentFields === null) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage:
        "Default environment variables are only available for cloud projects.",
    });
  }
  if (deploymentFields.deploymentType === "anonymous") {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage:
        "Default environment variables are not available for anonymous deployments.",
    });
  }
  const dtype = resolveDefaultEnvDtype(deploymentFields.deploymentType);
  const { projectId } = await fetchTeamAndProject(
    ctx,
    deploymentFields.deploymentName,
  );
  return defaultEnvBackend(ctx, projectId, dtype);
}

function resolveDefaultEnvDtype(
  deploymentType: Exclude<DeploymentType, "anonymous">,
): CloudDeploymentType {
  if (deploymentType === "local") return "dev";
  return deploymentType;
}
