import * as dotenv from "dotenv";
import { Command, Option } from "@commander-js/extra-typings";
import {
  Context,
  logVerbose,
  oneoffContext,
  showSpinner,
} from "../bundler/context.js";
import { handleManuallySetUrlAndAdminKey } from "./configure.js";
import { devAgainstDeployment } from "./lib/dev.js";
import { normalizeDevOptions } from "./lib/command.js";
import { deployToDeployment } from "./lib/deploy2.js";
import {
  CONVEX_DEPLOY_KEY_ENV_VAR_NAME,
  CONVEX_SELF_HOST_ADMIN_KEY_VAR_NAME,
  CONVEX_SELF_HOST_URL_VAR_NAME,
  ENV_VAR_FILE_PATH,
} from "./lib/utils/utils.js";
import { CONVEX_DEPLOYMENT_VAR_NAME } from "./lib/deployment.js";
import { runInDeployment } from "./lib/run.js";
import { importIntoDeployment } from "./lib/convexImport.js";
import { exportFromDeployment } from "./lib/convexExport.js";
import { logsForDeployment } from "./lib/logs.js";
import { functionSpecForDeployment } from "./lib/functionSpec.js";
import { dataInDeployment } from "./lib/data.js";
import {
  envGetInDeployment,
  envSetInDeployment,
  envListInDeployment,
  envRemoveInDeployment,
} from "./lib/env.js";
import { runNetworkTestOnUrl, withTimeout } from "./lib/networkTest.js";

export const selfHost = new Command("self-host");

selfHost
  .command("dev")
  .summary("Develop against a deployment, watching for changes")
  .description(
    "Develop against a deployment, watching for changes\n\n" +
      "  1. Connects to a deployment with the provided url and admin key\n" +
      "  2. Updates generated types and pushes code to the configured dev deployment\n" +
      "  3. Runs the provided function (if `--run` is used)\n" +
      "  4. Watches for file changes, and repeats step 2\n",
  )
  .allowExcessArguments(false)
  .addDevOptions()
  .addSelfHostOptions()
  .action(async (cmdOptions) => {
    const ctx = oneoffContext();
    process.on("SIGINT", async () => {
      logVerbose(ctx, "Received SIGINT, cleaning up...");
      await ctx.flushAndExit(-2);
    });

    const devOptions = await normalizeDevOptions(ctx, cmdOptions);

    const credentials = await selfHostCredentials(ctx, true, cmdOptions);

    await handleManuallySetUrlAndAdminKey(ctx, {
      url: credentials.url,
      adminKey: credentials.adminKey,
    });

    await devAgainstDeployment(ctx, credentials, devOptions);
  });

selfHost
  .command("deploy")
  .summary("Deploy to your deployment")
  .description(
    "Deploy to your deployment.\n\n" +
      "Unlike other `npx convex self-host` commands, " +
      "`npx convex self-host deploy` does not automatically look in " +
      ".env or .env.local files for url and admin key environment variables.",
  )
  .allowExcessArguments(false)
  .addDeployOptions()
  .addSelfHostOptions()
  .action(async (cmdOptions) => {
    const ctx = oneoffContext();

    const credentials = await selfHostCredentials(ctx, false, cmdOptions);

    await deployToDeployment(ctx, credentials, cmdOptions);
  });

async function getConfiguredCredentialsFromEnvVar(
  ctx: Context,
  envPath: string | undefined,
  includeDefaultEnv: boolean,
): Promise<{
  url?: string | undefined;
  adminKey?: string | undefined;
}> {
  if (envPath) {
    dotenv.config({ path: envPath });
  }
  if (includeDefaultEnv) {
    dotenv.config({ path: ENV_VAR_FILE_PATH });
    dotenv.config();
  }
  if (process.env[CONVEX_DEPLOYMENT_VAR_NAME]) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `Cloud-hosted deployment "${process.env[CONVEX_DEPLOYMENT_VAR_NAME]}" is already set.
      For self-hosted deployments via \`npx convex self-host\`, unset the "${CONVEX_DEPLOYMENT_VAR_NAME}" environment variable.`,
    });
  }
  if (process.env[CONVEX_DEPLOY_KEY_ENV_VAR_NAME]) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `Cloud-hosted deploy key is already set.
      For self-hosted deployments via \`npx convex self-host\`, unset the "${CONVEX_DEPLOY_KEY_ENV_VAR_NAME}" environment variable.`,
    });
  }
  const url = process.env[CONVEX_SELF_HOST_URL_VAR_NAME];
  const adminKey = process.env[CONVEX_SELF_HOST_ADMIN_KEY_VAR_NAME];
  return { url, adminKey };
}

async function selfHostCredentials(
  ctx: Context,
  includeDefaultEnv: boolean,
  cmdOptions: {
    env?: string;
    adminKey?: string;
    url?: string;
  },
) {
  const envVarCredentials = await getConfiguredCredentialsFromEnvVar(
    ctx,
    cmdOptions.env,
    includeDefaultEnv,
  );
  const urlOverride = cmdOptions.url ?? envVarCredentials.url;
  const adminKeyOverride = cmdOptions.adminKey ?? envVarCredentials.adminKey;
  if (urlOverride !== undefined && adminKeyOverride !== undefined) {
    return { url: urlOverride, adminKey: adminKeyOverride };
  }
  return await ctx.crash({
    exitCode: 1,
    errorType: "fatal",
    printedMessage:
      "Connect to self-hosted deployment with a url and admin key, " +
      "via flags --url and --admin-key, or environment variables " +
      `"${CONVEX_SELF_HOST_URL_VAR_NAME}" and "${CONVEX_SELF_HOST_ADMIN_KEY_VAR_NAME}"`,
  });
}

selfHost
  .command("run")
  .description("Run a function (query, mutation, or action) on your deployment")
  .allowExcessArguments(false)
  .addRunOptions()
  .addSelfHostOptions()
  .showHelpAfterError()
  .action(async (functionName, argsString, options) => {
    const ctx = oneoffContext();

    const { adminKey, url: deploymentUrl } = await selfHostCredentials(
      ctx,
      true,
      options,
    );

    await runInDeployment(ctx, {
      deploymentUrl,
      adminKey,
      functionName,
      argsString: argsString ?? "{}",
      componentPath: options.component,
      identityString: options.identity,
      push: !!options.push,
      watch: !!options.watch,
      typecheck: options.typecheck,
      typecheckComponents: options.typecheckComponents,
      codegen: options.codegen === "enable",
      liveComponentSources: !!options.liveComponentSources,
    });
  });

selfHost
  .command("import")
  .summary("Import data from a file to your deployment")
  .description(
    "Import data from a file to your Convex deployment.\n\n" +
      "  From a snapshot: `npx convex import snapshot.zip`\n" +
      "  For a single table: `npx convex import --table tableName file.json`",
  )
  .allowExcessArguments(false)
  .addImportOptions()
  .addSelfHostOptions()
  .showHelpAfterError()
  .action(async (filePath, options) => {
    const ctx = oneoffContext();

    const { adminKey, url: deploymentUrl } = await selfHostCredentials(
      ctx,
      true,
      options,
    );

    await importIntoDeployment(ctx, filePath, {
      ...options,
      deploymentUrl,
      adminKey,
      deploymentNotice: "",
      snapshotImportDashboardLink: undefined,
    });
  });

selfHost
  .command("export")
  .summary("Export data from your deployment to a ZIP file")
  .description(
    "Export data, and optionally file storage, from your Convex deployment to a ZIP file.",
  )
  .allowExcessArguments(false)
  .addExportOptions()
  .addSelfHostOptions()
  .showHelpAfterError()
  .action(async (options) => {
    const ctx = oneoffContext();

    const { adminKey, url: deploymentUrl } = await selfHostCredentials(
      ctx,
      true,
      options,
    );

    await exportFromDeployment(ctx, {
      ...options,
      deploymentUrl,
      adminKey,
      deploymentNotice: "",
      snapshotExportDashboardLink: undefined,
    });
  });

async function selfHostEnvDeployment(
  ctx: Context,
  options: {
    env?: string;
    adminKey?: string;
    url?: string;
  },
) {
  const deployment = await selfHostCredentials(ctx, true, options);
  return {
    deploymentUrl: deployment.url,
    adminKey: deployment.adminKey,
    deploymentNotice: "",
  };
}

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
    const deployment = await selfHostEnvDeployment(ctx, options);
    await envSetInDeployment(ctx, deployment, originalName, originalValue);
  });

const envGet = new Command("get")
  .arguments("<name>")
  .summary("Print a variable's value")
  .description("Print a variable's value: `npx convex env get NAME`")
  .configureHelp({ showGlobalOptions: true })
  .allowExcessArguments(false)
  .action(async (envVarName, _options, cmd) => {
    const ctx = oneoffContext();
    const options = cmd.optsWithGlobals();
    const deployment = await selfHostEnvDeployment(ctx, options);
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
    const deployment = await selfHostEnvDeployment(ctx, options);
    await envRemoveInDeployment(ctx, deployment, name);
  });

const envList = new Command("list")
  .summary("List all variables")
  .description("List all variables: `npx convex env list`")
  .configureHelp({ showGlobalOptions: true })
  .allowExcessArguments(false)
  .action(async (_options, cmd) => {
    const ctx = oneoffContext();
    const options = cmd.optsWithGlobals();
    const deployment = await selfHostEnvDeployment(ctx, options);
    await envListInDeployment(ctx, deployment);
  });

selfHost
  .command("env")
  .summary("Set and view environment variables")
  .description(
    "Set and view environment variables on your deployment\n\n" +
      "  Set a variable: `npx convex env set NAME value`\n" +
      "  Unset a variable: `npx convex env remove NAME`\n" +
      "  List all variables: `npx convex env list`\n" +
      "  Print a variable's value: `npx convex env get NAME`",
  )
  .addCommand(envSet)
  .addCommand(envGet)
  .addCommand(envRemove)
  .addCommand(envList)
  .addHelpCommand(false)
  .addSelfHostOptions();

selfHost
  .command("data")
  .summary("List tables and print data from your database")
  .description(
    "Inspect your Convex deployment's database.\n\n" +
      "  List tables: `npx convex data`\n" +
      "  List documents in a table: `npx convex data tableName`",
  )
  .allowExcessArguments(false)
  .addDataOptions()
  .addSelfHostOptions()
  .showHelpAfterError()
  .action(async (tableName, options) => {
    const ctx = oneoffContext();

    const credentials = await selfHostCredentials(ctx, true, options);

    await dataInDeployment(ctx, {
      deploymentUrl: credentials.url,
      adminKey: credentials.adminKey,
      deploymentNotice: "",
      tableName,
      ...options,
    });
  });

selfHost
  .command("function-spec")
  .summary("List function metadata from your deployment")
  .description("List argument and return values to your Convex functions.")
  .allowExcessArguments(false)
  .addOption(new Option("--file", "Output as JSON to a file."))
  .addSelfHostOptions()
  .showHelpAfterError()
  .action(async (options) => {
    const ctx = oneoffContext();
    const credentials = await selfHostCredentials(ctx, true, options);

    await functionSpecForDeployment(ctx, {
      deploymentUrl: credentials.url,
      adminKey: credentials.adminKey,
      file: !!options.file,
    });
  });

selfHost
  .command("logs")
  .summary("Watch logs from your deployment")
  .description("Stream function logs from your Convex deployment.")
  .allowExcessArguments(false)
  .addLogsOptions()
  .addSelfHostOptions()
  .showHelpAfterError()
  .action(async (cmdOptions) => {
    const ctx = oneoffContext();

    const credentials = await selfHostCredentials(ctx, true, cmdOptions);
    await logsForDeployment(ctx, credentials, {
      history: cmdOptions.history,
      success: cmdOptions.success,
      deploymentNotice: "",
    });
  });

selfHost
  .command("network-test")
  .description("Run a network test to Convex's servers")
  .allowExcessArguments(false)
  .addNetworkTestOptions()
  .addSelfHostOptions()
  .action(async (options) => {
    const ctx = oneoffContext();
    const timeoutSeconds = options.timeout
      ? Number.parseFloat(options.timeout)
      : 30;
    await withTimeout(
      ctx,
      "Network test",
      timeoutSeconds * 1000,
      runNetworkTest(ctx, options),
    );
  });

async function runNetworkTest(
  ctx: Context,
  options: {
    url?: string | undefined;
    timeout?: string;
    ipFamily?: string;
    speedTest?: boolean;
  },
) {
  showSpinner(ctx, "Performing network test...");
  const credentials = await selfHostCredentials(ctx, true, {
    // adminKey is unused for network test, so it doesn't need to be provided.
    // give a default to prevent errors
    adminKey: "",
    ...options,
  });
  const url = credentials.url;
  await runNetworkTestOnUrl(ctx, url, options);
}
