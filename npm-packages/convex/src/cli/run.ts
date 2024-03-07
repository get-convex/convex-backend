import { Command, Option } from "@commander-js/extra-typings";
import { logFailure, oneoffContext } from "../bundler/context.js";
import { watchAndPush } from "./dev.js";
import {
  fetchDeploymentCredentialsProvisionProd,
  deploymentSelectionFromOptions,
} from "./lib/api.js";
import { actionDescription } from "./lib/command.js";
import { runFunctionAndLog, subscribeAndLog } from "./lib/run.js";
import { ensureHasConvexDependency } from "./lib/utils.js";

export const run = new Command("run")
  .description("Run a function (query, mutation, or action) on your deployment")
  .argument(
    "functionName",
    "identifier of the function to run, like `listMessages` or `dir/file:myFunction`",
  )
  .argument(
    "[args]",
    "JSON-formatted arguments object to pass to the function.",
  )
  .option(
    "-w, --watch",
    "Watch a query, printing its result if the underlying data changes. Given function must be a query.",
  )
  .option("--push", "Push code to deployment before running the function.")
  // For backwards compatibility we still support --no-push which is a noop
  .addOption(new Option("--no-push").hideHelp())
  .addDeploymentSelectionOptions(actionDescription("Run the function on"))
  // Options for the implicit dev deploy
  .addOption(
    new Option(
      "--typecheck <mode>",
      `Whether to check TypeScript files with \`tsc --noEmit\`.`,
    )
      .choices(["enable", "try", "disable"] as const)
      .default("try" as const),
  )
  .addOption(
    new Option("--codegen <mode>", "Regenerate code in `convex/_generated/`")
      .choices(["enable", "disable"] as const)
      .default("enable" as const),
  )

  .showHelpAfterError()
  .action(async (functionName, argsString, options) => {
    const ctx = oneoffContext;

    const deploymentSelection = deploymentSelectionFromOptions(options);

    const {
      adminKey,
      url: deploymentUrl,
      deploymentType,
    } = await fetchDeploymentCredentialsProvisionProd(ctx, deploymentSelection);

    await ensureHasConvexDependency(ctx, "run");

    const args = argsString ? JSON.parse(argsString) : {};

    if (deploymentType === "prod" && options.push) {
      logFailure(
        ctx,
        `\`convex run\` doesn't support pushing functions to prod deployments. ` +
          `Remove the --push flag. To push to production use \`npx convex deploy\`.`,
      );
      return await ctx.crash(1, "fatal");
    }

    if (options.push) {
      await watchAndPush(
        ctx,
        {
          adminKey,
          verbose: false,
          dryRun: false,
          typecheck: options.typecheck,
          debug: false,
          codegen: options.codegen === "enable",
          url: deploymentUrl,
        },
        {
          once: true,
          traceEvents: false,
          untilSuccess: true,
        },
      );
    }

    if (options.watch) {
      return await subscribeAndLog(
        ctx,
        deploymentUrl,
        adminKey,
        functionName,
        args,
      );
    }
    return await runFunctionAndLog(
      ctx,
      deploymentUrl,
      adminKey,
      functionName,
      args,
    );
  });
