import { Command } from "@commander-js/extra-typings";
import { logFinishedStep, oneoffContext } from "../bundler/context.js";
import { getConfiguredDeployment } from "./lib/utils/utils.js";
import { deploymentCredentialsOrConfigure } from "./configure.js";

export const disableLocalDeployments = new Command("disable-local-deployments")
  .description(
    "Disable local deployments on this machine until a future release when this feature is more stable.",
  )
  .allowExcessArguments(false)
  .action(async () => {
    const ctx = oneoffContext();

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
