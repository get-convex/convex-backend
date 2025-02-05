import { Command } from "@commander-js/extra-typings";
import { logVerbose, oneoffContext, OneoffCtx } from "../bundler/context.js";
import { handleManuallySetUrlAndAdminKey } from "./configure.js";
import { devAgainstDeployment } from "./lib/dev.js";
import { normalizeDevOptions } from "./lib/command.js";
import { getConfiguredCredentialsFromEnvVar } from "./lib/deployment.js";
import { storeAdminKeyEnvVar } from "./lib/api.js";
import { deployToDeployment } from "./lib/deploy2.js";
import { runInDeployment } from "./lib/run.js";

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
  .option(
    "--admin-key <adminKey>",
    "An admin key for the deployment. Can alternatively be set as `CONVEX_DEPLOY_KEY` environment variable.",
  )
  .option(
    "--url <url>",
    "The url of the deployment. Can alternatively be set as `CONVEX_SELF_HOST_DEPLOYMENT_URL` environment variable.",
  )
  .action(async (cmdOptions) => {
    const ctx = oneoffContext();
    process.on("SIGINT", async () => {
      logVerbose(ctx, "Received SIGINT, cleaning up...");
      await ctx.flushAndExit(-2);
    });

    const devOptions = await normalizeDevOptions(ctx, cmdOptions);

    const credentials = await selfHostCredentials(ctx, true, cmdOptions);

    await devAgainstDeployment(ctx, credentials, devOptions);
  });

selfHost
  .command("deploy")
  .summary("Deploy to your deployment")
  .description("Deploy to your deployment.")
  .allowExcessArguments(false)
  .addDeployOptions()
  .option(
    "--admin-key <adminKey>",
    "An admin key for the deployment. Can alternatively be set as `CONVEX_DEPLOY_KEY` environment variable.",
  )
  .option(
    "--url <url>",
    "The url of the deployment. Can alternatively be set as `CONVEX_SELF_HOST_DEPLOYMENT_URL` environment variable.",
  )
  .action(async (cmdOptions) => {
    const ctx = oneoffContext();

    storeAdminKeyEnvVar(cmdOptions.adminKey);
    const credentials = await selfHostCredentials(ctx, false, cmdOptions);

    await deployToDeployment(ctx, credentials, cmdOptions);
  });

async function selfHostCredentials(
  ctx: OneoffCtx,
  writeEnvVarsToFile: boolean,
  cmdOptions: {
    adminKey?: string;
    url?: string;
  },
) {
  const envVarCredentials = getConfiguredCredentialsFromEnvVar();
  const urlOverride = cmdOptions.url ?? envVarCredentials.url;
  const adminKeyOverride = cmdOptions.adminKey ?? envVarCredentials.adminKey;
  if (urlOverride !== undefined && adminKeyOverride !== undefined) {
    if (writeEnvVarsToFile) {
      await handleManuallySetUrlAndAdminKey(ctx, {
        url: urlOverride,
        adminKey: adminKeyOverride,
      });
    }
    return { url: urlOverride, adminKey: adminKeyOverride };
  }
  return await ctx.crash({
    exitCode: 1,
    errorType: "fatal",
    printedMessage:
      "Connect to self-hosted deployment with a url and admin key, " +
      "via flags --url and --admin-key, " +
      "or environment variables CONVEX_SELF_HOST_DEPLOYMENT_URL and CONVEX_DEPLOY_KEY",
  });
}

selfHost
  .command("run")
  .description("Run a function (query, mutation, or action) on your deployment")
  .allowExcessArguments(false)
  .addRunOptions()
  .option(
    "--admin-key <adminKey>",
    "An admin key for the deployment. Can alternatively be set as `CONVEX_DEPLOY_KEY` environment variable.",
  )
  .option(
    "--url <url>",
    "The url of the deployment. Can alternatively be set as `CONVEX_SELF_HOST_DEPLOYMENT_URL` environment variable.",
  )
  .showHelpAfterError()
  .action(async (functionName, argsString, options) => {
    const ctx = oneoffContext();

    const { adminKey, url: deploymentUrl } = await selfHostCredentials(
      ctx,
      false,
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
