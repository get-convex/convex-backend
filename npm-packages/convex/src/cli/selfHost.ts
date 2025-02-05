import * as dotenv from "dotenv";
import { Command } from "@commander-js/extra-typings";
import { Context, logVerbose, oneoffContext } from "../bundler/context.js";
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
