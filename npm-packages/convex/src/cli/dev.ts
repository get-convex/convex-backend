import { Command, Option } from "@commander-js/extra-typings";
import { logVerbose, oneoffContext } from "../bundler/context.js";
import { deploymentCredentialsOrConfigure } from "./configure.js";
import { checkAuthorization, performLogin } from "./lib/login.js";
import { usageStateWarning } from "./lib/usage.js";
import { normalizeDevOptions } from "./lib/command.js";
import { devAgainstDeployment } from "./lib/dev.js";

export const dev = new Command("dev")
  .summary("Develop against a dev deployment, watching for changes")
  .description(
    "Develop against a dev deployment, watching for changes\n\n" +
      "  1. Configures a new or existing project (if needed)\n" +
      "  2. Updates generated types and pushes code to the configured dev deployment\n" +
      "  3. Runs the provided function (if `--run` is used)\n" +
      "  4. Watches for file changes, and repeats step 2\n",
  )
  .allowExcessArguments(false)
  .addDevOptions()
  .addOption(
    new Option(
      "--configure [choice]",
      "Ignore existing configuration and configure new or existing project, interactively or set by --team <team_slug>, --project <project_slug>, and --dev-deployment local|cloud",
    )
      .choices(["new", "existing"] as const)
      .conflicts(["--local", "--cloud"]),
  )
  .addOption(
    new Option(
      "--team <team_slug>",
      "The team you'd like to use for this project",
    ).hideHelp(),
  )
  .addOption(
    new Option(
      "--project <project_slug>",
      "The name of the project you'd like to configure",
    ).hideHelp(),
  )
  .addOption(
    new Option(
      "--dev-deployment <mode>",
      "Use a local or cloud deployment for dev for this project",
    )
      .choices(["cloud", "local"] as const)
      .conflicts(["--prod"])
      .hideHelp(),
  )
  .addOption(
    new Option(
      "--prod",
      "Develop live against this project's production deployment.",
    )
      .default(false)
      .hideHelp(),
  )
  .addOption(new Option("--skip-push").default(false).hideHelp())
  .addOption(new Option("--admin-key <adminKey>").hideHelp())
  .addOption(new Option("--url <url>").hideHelp())
  // Options for testing
  .addOption(new Option("--override-auth-url <url>").hideHelp())
  .addOption(new Option("--override-auth-client <id>").hideHelp())
  .addOption(new Option("--override-auth-username <username>").hideHelp())
  .addOption(new Option("--override-auth-password <password>").hideHelp())
  .addOption(new Option("--local-cloud-port <port>").hideHelp())
  .addOption(new Option("--local-site-port <port>").hideHelp())
  .addOption(new Option("--local-backend-version <version>").hideHelp())
  .addOption(new Option("--local-force-upgrade").default(false).hideHelp())
  .addOption(new Option("--partition-id <id>").hideHelp())
  .addOption(
    new Option(
      "--local",
      "Use local deployment regardless of last used backend. DB data will not be downloaded from any cloud deployment.",
    )
      .default(false)
      .conflicts(["--prod", "--url", "--admin-key", "--cloud"])
      .hideHelp(),
  )
  .addOption(
    new Option(
      "--cloud",
      "Use cloud deployment regardles of last used backend. DB data will not be uploaded from local.",
    )
      .default(false)
      .conflicts(["--prod", "--url", "--admin-key", "--local"])
      .hideHelp(),
  )
  .showHelpAfterError()
  .action(async (cmdOptions) => {
    const ctx = oneoffContext();
    process.on("SIGINT", async () => {
      logVerbose(ctx, "Received SIGINT, cleaning up...");
      await ctx.flushAndExit(-2);
    });

    const devOptions = await normalizeDevOptions(ctx, cmdOptions);

    if (cmdOptions.configure === undefined) {
      if (cmdOptions.team || cmdOptions.project || cmdOptions.devDeployment)
        return await ctx.crash({
          exitCode: 1,
          errorType: "fatal",
          printedMessage:
            "`--team, --project, and --dev-deployment can can only be used with `--configure`.",
        });
    }

    const localOptions: {
      ports?: { cloud: number; site: number };
      backendVersion?: string | undefined;
      forceUpgrade: boolean;
    } = { forceUpgrade: false };
    if (!cmdOptions.local && cmdOptions.devDeployment !== "local") {
      if (
        cmdOptions.localCloudPort !== undefined ||
        cmdOptions.localSitePort !== undefined ||
        cmdOptions.localBackendVersion !== undefined ||
        cmdOptions.localForceUpgrade === true
      ) {
        return await ctx.crash({
          exitCode: 1,
          errorType: "fatal",
          printedMessage:
            "`--local-*` options can only be used with `--configure --dev-deployment local` or `--local`.",
        });
      }
    } else {
      if (cmdOptions.localCloudPort !== undefined) {
        if (cmdOptions.localSitePort === undefined) {
          return await ctx.crash({
            exitCode: 1,
            errorType: "fatal",
            printedMessage:
              "`--local-cloud-port` requires `--local-site-port` to be set.",
          });
        }
        localOptions["ports"] = {
          cloud: parseInt(cmdOptions.localCloudPort),
          site: parseInt(cmdOptions.localSitePort),
        };
      }
      localOptions["backendVersion"] = cmdOptions.localBackendVersion;
      localOptions["forceUpgrade"] = cmdOptions.localForceUpgrade;
    }

    if (!cmdOptions.url || !cmdOptions.adminKey) {
      if (!(await checkAuthorization(ctx, false))) {
        await performLogin(ctx, cmdOptions);
      }
    }

    const partitionId = cmdOptions.partitionId
      ? parseInt(cmdOptions.partitionId)
      : undefined;
    const configure =
      cmdOptions.configure === true ? "ask" : (cmdOptions.configure ?? null);
    const credentials = await deploymentCredentialsOrConfigure(
      ctx,
      configure,
      {
        ...cmdOptions,
        localOptions,
      },
      partitionId,
    );

    await usageStateWarning(ctx);

    if (cmdOptions.skipPush) {
      return;
    }

    await devAgainstDeployment(ctx, credentials, devOptions);
  });
