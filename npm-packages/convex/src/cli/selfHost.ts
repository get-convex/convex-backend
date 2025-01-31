import { Command } from "@commander-js/extra-typings";
import { logVerbose, oneoffContext, OneoffCtx } from "../bundler/context.js";
import { handleManuallySetUrlAndAdminKey } from "./configure.js";
import { devAgainstDeployment } from "./lib/dev.js";
import { normalizeDevOptions } from "./lib/command.js";
import { getConfiguredCredentialsFromEnvVar } from "./lib/deployment.js";

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

    const credentials = await selfHostCredentials(ctx, cmdOptions);

    await devAgainstDeployment(ctx, credentials, devOptions);
  });

async function selfHostCredentials(
  ctx: OneoffCtx,
  cmdOptions: {
    adminKey?: string;
    url?: string;
  },
) {
  const envVarCredentials = getConfiguredCredentialsFromEnvVar();
  const urlOverride = cmdOptions.url ?? envVarCredentials.url;
  const adminKeyOverride = cmdOptions.adminKey ?? envVarCredentials.adminKey;
  if (urlOverride !== undefined && adminKeyOverride !== undefined) {
    const credentials = await handleManuallySetUrlAndAdminKey(ctx, {
      url: urlOverride,
      adminKey: adminKeyOverride,
    });
    return { ...credentials };
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
