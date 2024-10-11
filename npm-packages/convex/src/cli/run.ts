import { Command, Option } from "@commander-js/extra-typings";
import { oneoffContext } from "../bundler/context.js";
import { watchAndPush } from "./dev.js";
import {
  fetchDeploymentCredentialsProvisionProd,
  deploymentSelectionFromOptions,
} from "./lib/api.js";
import { actionDescription } from "./lib/command.js";
import { runFunctionAndLog, subscribeAndLog } from "./lib/run.js";
import { ensureHasConvexDependency } from "./lib/utils/utils.js";

export const run = new Command("run")
  .description("Run a function (query, mutation, or action) on your deployment")
  .allowExcessArguments(false)
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
  .option(
    "--typecheck-components",
    "Check TypeScript files within component implementations with `tsc --noEmit`.",
    false,
  )
  .addOption(
    new Option("--codegen <mode>", "Regenerate code in `convex/_generated/`")
      .choices(["enable", "disable"] as const)
      .default("enable" as const),
  )
  .addOption(
    new Option(
      "--component <path>",
      "Path to the component in the component tree defined in convex.config.ts. " +
        "Components are a beta feature. This flag is unstable and may change in subsequent releases.",
    ),
  )
  .addOption(new Option("--live-component-sources").hideHelp())

  .showHelpAfterError()
  .action(async (functionName, argsString, options) => {
    const ctx = oneoffContext();

    const deploymentSelection = deploymentSelectionFromOptions(options);

    const {
      adminKey,
      url: deploymentUrl,
      deploymentType,
    } = await fetchDeploymentCredentialsProvisionProd(ctx, deploymentSelection);

    await ensureHasConvexDependency(ctx, "run");

    const args = argsString ? JSON.parse(argsString) : {};

    if (deploymentType === "prod" && options.push) {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage:
          `\`convex run\` doesn't support pushing functions to prod deployments. ` +
          `Remove the --push flag. To push to production use \`npx convex deploy\`.`,
      });
    }

    if (options.push) {
      await watchAndPush(
        ctx,
        {
          adminKey,
          verbose: false,
          dryRun: false,
          typecheck: options.typecheck,
          typecheckComponents: options.typecheckComponents,
          debug: false,
          codegen: options.codegen === "enable",
          url: deploymentUrl,
          liveComponentSources: !!options.liveComponentSources,
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
        options.component,
      );
    }
    return await runFunctionAndLog(
      ctx,
      deploymentUrl,
      adminKey,
      functionName,
      args,
      options.component,
    );
  });
