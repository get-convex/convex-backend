import { Command } from "@commander-js/extra-typings";
import { logFinishedStep, oneoffContext } from "../bundler/context.js";
import { getConfiguredDeployment } from "./lib/utils/utils.js";
import { deploymentCredentialsOrConfigure } from "./configure.js";
import {
  modifyGlobalConfig,
  readGlobalConfig,
} from "./lib/utils/globalConfig.js";

export const disableLocalDeployments = new Command("disable-local-deployments")
  .description(
    "Stop using a local deployment for the current project, or globally disable local depoyments with --global",
  )
  .option(
    "--global",
    "Disable local deployments on this machine until a future release when this feature is more stable.",
  )
  .option("--undo-global", "Re-enable local deployments on this machine.")
  .allowExcessArguments(false)
  .action(async (cmdOptions) => {
    const ctx = oneoffContext();

    if (cmdOptions.undoGlobal) {
      return disableLocalDeploymentsGloballyUntilBetaOver(true);
    }
    if (cmdOptions.global) {
      return disableLocalDeploymentsGloballyUntilBetaOver(
        !!cmdOptions.undoGlobal,
      );
    }

    const { type } = await getConfiguredDeployment(ctx);
    if (type !== "local") {
      logFinishedStep(ctx, "Local development is already not being used.");
      return;
    }

    await deploymentCredentialsOrConfigure(ctx, null, {
      prod: false,
      localOptions: {
        forceUpgrade: false,
      },
      cloud: true,
    });

    logFinishedStep(
      ctx,
      "You are no longer using a local deployment for development.",
    );
  });

async function disableLocalDeploymentsGloballyUntilBetaOver(
  reenable: boolean,
): Promise<void> {
  const ctx = oneoffContext();

  // Ensure this is not used in CI or scripts, since it has global effects and will be deprecated
  // in the future.
  if (!process.stdin.isTTY) {
    return ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage:
        "`disable-local-deployments --global` is not for scripting, it is temporary and only for interactive use.",
    });
  }
  const config = readGlobalConfig(ctx);
  if (config === null) {
    return ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: "Log in first with `npx convex login",
    });
  }

  if (reenable) {
    if (
      !("optOutOfLocalDevDeploymentsUntilBetaOver" in config) ||
      !config.optOutOfLocalDevDeploymentsUntilBetaOver
    ) {
      logFinishedStep(
        ctx,
        "You are already opted into allowing local deployents on this machine.",
      );
      return;
    }
    await modifyGlobalConfig(ctx, {
      ...config,
      optOutOfLocalDevDeploymentsUntilBetaOver: false,
    });

    logFinishedStep(
      ctx,
      "You have been opted back into allowing local deployents on this machine.",
    );
    return;
  }

  if (
    "optOutOfLocalDevDeploymentsUntilBetaOver" in config &&
    config.optOutOfLocalDevDeploymentsUntilBetaOver
  ) {
    logFinishedStep(
      ctx,
      "You are already opted out of local deployents on this machine.",
    );
    return;
  }
  await modifyGlobalConfig(ctx, {
    ...config,
    optOutOfLocalDevDeploymentsUntilBetaOver: true,
  });

  logFinishedStep(
    ctx,
    "You have been opted out of local deployents on this machine until the beta is over. Run `npx convex disable-local-deployments --undo-global` to opt back in.",
  );
}
