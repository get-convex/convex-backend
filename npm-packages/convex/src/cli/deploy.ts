import chalk from "chalk";
import { Command, Option } from "@commander-js/extra-typings";
import inquirer from "inquirer";
import {
  Context,
  logFinishedStep,
  logMessage,
  oneoffContext,
  showSpinner,
} from "../bundler/context.js";
import {
  fetchDeploymentCredentialsWithinCurrentProject,
  deploymentSelectionFromOptions,
  projectSelection,
  storeAdminKeyEnvVar,
} from "./lib/api.js";
import {
  gitBranchFromEnvironment,
  isNonProdBuildEnvironment,
  suggestedEnvVarName,
} from "./lib/envvars.js";
import { PushOptions } from "./lib/push.js";
import {
  CONVEX_DEPLOY_KEY_ENV_VAR_NAME,
  bigBrainAPI,
  getConfiguredDeploymentName,
  readAdminKeyFromEnvVar,
} from "./lib/utils.js";
import { spawnSync } from "child_process";
import { runFunctionAndLog } from "./lib/run.js";
import { usageStateWarning } from "./lib/usage.js";
import {
  deploymentTypeFromAdminKey,
  getConfiguredDeploymentFromEnvVar,
  isPreviewDeployKey,
} from "./lib/deployment.js";
import { runPush } from "./lib/components.js";

export const deploy = new Command("deploy")
  .summary("Deploy to your prod deployment")
  .description(
    "Deploy to your deployment. By default, this deploys to your prod deployment.\n\n" +
      "Deploys to a preview deployment if the `CONVEX_DEPLOY_KEY` environment variable is set to a Preview Deploy Key.",
  )
  .option("-v, --verbose", "Show full listing of changes")
  .option(
    "--dry-run",
    "Print out the generated configuration without deploying to your Convex deployment",
  )
  .option("-y, --yes", "Skip confirmation prompt when running locally")
  .addOption(
    new Option(
      "--typecheck <mode>",
      `Whether to check TypeScript files with \`tsc --noEmit\` before deploying.`,
    )
      .choices(["enable", "try", "disable"] as const)
      .default("try" as const),
  )
  .addOption(
    new Option(
      "--codegen <mode>",
      "Whether to regenerate code in `convex/_generated/` before pushing.",
    )
      .choices(["enable", "disable"] as const)
      .default("enable" as const),
  )
  .addOption(
    new Option(
      "--cmd <command>",
      "Command to run as part of deploying your app (e.g. `vite build`). This command can depend on the environment variables specified in `--cmd-url-env-var-name` being set.",
    ),
  )
  .addOption(
    new Option(
      "--cmd-url-env-var-name <name>",
      "Environment variable name to set Convex deployment URL (e.g. `VITE_CONVEX_URL`) when using `--cmd`",
    ),
  )
  .addOption(
    new Option(
      "--preview-run <functionName>",
      "Function to run if deploying to a preview deployment. This is ignored if deploying to a production deployment.",
    ),
  )
  .addOption(
    new Option(
      "--preview-create <name>",
      "The name to associate with this deployment if deploying to a newly created preview deployment. Defaults to the current Git branch name in Vercel, Netlify and Github CI. This is ignored if deploying to a production deployment.",
    ).conflicts("preview-name"),
  )
  .addOption(
    new Option(
      "--check-build-environment <mode>",
      "Whether to check for a non-production build environment before deploying to a production Convex deployment.",
    )
      .choices(["enable", "disable"] as const)
      .default("enable" as const)
      .hideHelp(),
  )
  .addOption(new Option("--debug-bundle-path <path>").hideHelp())
  .addOption(new Option("--debug").hideHelp())
  // Hidden options to pass in admin key and url for tests and local development
  .addOption(new Option("--admin-key <adminKey>").hideHelp())
  .addOption(new Option("--url <url>").hideHelp())
  .addOption(new Option("--write-push-request <writePushRequest>").hideHelp()) // Option used for tests in backend
  .addOption(
    new Option(
      "--preview-name <name>",
      "[deprecated] Use `--preview-create` instead. The name to associate with this deployment if deploying to a preview deployment.",
    )
      .hideHelp()
      .conflicts("preview-create"),
  )
  .showHelpAfterError()
  .action(async (cmdOptions) => {
    const ctx = oneoffContext;

    storeAdminKeyEnvVar(cmdOptions.adminKey);
    const configuredDeployKey = readAdminKeyFromEnvVar() ?? null;
    if (
      cmdOptions.checkBuildEnvironment === "enable" &&
      isNonProdBuildEnvironment() &&
      configuredDeployKey !== null &&
      deploymentTypeFromAdminKey(configuredDeployKey) === "prod"
    ) {
      await ctx.crash({
        exitCode: 1,
        errorType: "invalid filesystem data",
        printedMessage: `Detected a non-production build environment and "${CONVEX_DEPLOY_KEY_ENV_VAR_NAME}" for a production Convex deployment.\n
          This is probably unintentional.
          `,
      });
    }

    await usageStateWarning(ctx);

    if (
      configuredDeployKey !== null &&
      isPreviewDeployKey(configuredDeployKey)
    ) {
      if (cmdOptions.previewName !== undefined) {
        await ctx.crash({
          exitCode: 1,
          errorType: "fatal",
          printedMessage:
            "The `--preview-name` flag has been deprecated in favor of `--preview-create`. Please re-run the command using `--preview-create` instead.",
        });
      }
      await deployToNewPreviewDeployment(ctx, {
        ...cmdOptions,
        configuredDeployKey,
      });
    } else {
      await deployToExistingDeployment(ctx, cmdOptions);
    }
  });

async function deployToNewPreviewDeployment(
  ctx: Context,
  options: {
    configuredDeployKey: string;
    dryRun?: boolean | undefined;
    previewCreate?: string | undefined;
    previewRun?: string | undefined;
    cmdUrlEnvVarName?: string | undefined;
    cmd?: string | undefined;
    verbose?: boolean | undefined;
    typecheck: "enable" | "try" | "disable";
    codegen: "enable" | "disable";

    debug?: boolean | undefined;
    debugBundlePath?: string | undefined;
  },
) {
  const previewName = options.previewCreate ?? gitBranchFromEnvironment();
  if (previewName === null) {
    await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage:
        "`npx convex deploy` to a preview deployment could not determine the preview name. Provide one using `--preview-create`",
    });
  }

  if (options.dryRun) {
    logFinishedStep(
      ctx,
      `Would have claimed preview deployment for "${previewName}"`,
    );
    await runCommand(ctx, {
      cmdUrlEnvVarName: options.cmdUrlEnvVarName,
      cmd: options.cmd,
      dryRun: !!options.dryRun,
      url: "https://<PREVIEW DEPLOYMENT>.convex.cloud",
    });
    logFinishedStep(
      ctx,
      `Would have deployed Convex functions to preview deployment for "${previewName}"`,
    );
    if (options.previewRun !== undefined) {
      logMessage(ctx, `Would have run function "${options.previewRun}"`);
    }
    return;
  }

  const data = await bigBrainAPI({
    ctx,
    method: "POST",
    url: "claim_preview_deployment",
    data: {
      projectSelection: await projectSelection(
        ctx,
        await getConfiguredDeploymentName(ctx),
        options.configuredDeployKey,
      ),
      identifier: previewName,
    },
  });

  const previewAdminKey = data.adminKey;
  const previewUrl = data.instanceUrl;

  await runCommand(ctx, { ...options, url: previewUrl });

  const pushOptions: PushOptions = {
    adminKey: previewAdminKey,
    verbose: !!options.verbose,
    dryRun: false,
    typecheck: options.typecheck,
    debug: !!options.debug,
    debugBundlePath: options.debugBundlePath,
    codegen: options.codegen === "enable",
    url: previewUrl,
    cleanupHandle: null,
  };
  showSpinner(ctx, `Deploying to ${previewUrl}...`);
  await runPush(oneoffContext, pushOptions);
  logFinishedStep(ctx, `Deployed Convex functions to ${previewUrl}`);

  if (options.previewRun !== undefined) {
    await runFunctionAndLog(
      ctx,
      previewUrl,
      previewAdminKey,
      options.previewRun,
      {},
      undefined,
      {
        onSuccess: () => {
          logFinishedStep(
            ctx,
            `Finished running function "${options.previewRun}"`,
          );
        },
      },
    );
  }
}

async function deployToExistingDeployment(
  ctx: Context,
  options: {
    verbose?: boolean | undefined;
    dryRun?: boolean | undefined;
    yes?: boolean | undefined;
    typecheck: "enable" | "try" | "disable";
    codegen: "enable" | "disable";
    cmd?: string | undefined;
    cmdUrlEnvVarName?: string | undefined;

    debugBundlePath?: string | undefined;
    debug?: boolean | undefined;
    adminKey?: string | undefined;
    url?: string | undefined;
    writePushRequest?: string | undefined;
  },
) {
  const deploymentSelection = deploymentSelectionFromOptions({
    ...options,
    prod: true,
  });
  const { name: configuredDeploymentName, type: configuredDeploymentType } =
    getConfiguredDeploymentFromEnvVar();
  const { adminKey, url, deploymentName, deploymentType } =
    await fetchDeploymentCredentialsWithinCurrentProject(
      ctx,
      deploymentSelection,
    );
  if (
    deploymentSelection.kind !== "deployKey" &&
    deploymentName !== undefined &&
    deploymentType !== undefined &&
    configuredDeploymentName !== null
  ) {
    const shouldPushToProd =
      deploymentName === configuredDeploymentName ||
      (options.yes ??
        (await askToConfirmPush(
          ctx,
          {
            configuredName: configuredDeploymentName,
            configuredType: configuredDeploymentType,
            requestedName: deploymentName,
            requestedType: deploymentType,
          },
          url,
        )));
    if (!shouldPushToProd) {
      await ctx.crash({
        exitCode: 1,
        printedMessage: null,
        errorType: "fatal",
      });
    }
  }

  await runCommand(ctx, { ...options, url });

  const pushOptions: PushOptions = {
    adminKey,
    verbose: !!options.verbose,
    dryRun: !!options.dryRun,
    typecheck: options.typecheck,
    debug: !!options.debug,
    debugBundlePath: options.debugBundlePath,
    codegen: options.codegen === "enable",
    url,
    writePushRequest: options.writePushRequest,
    cleanupHandle: null,
  };
  showSpinner(
    ctx,
    `Deploying to ${url}...${options.dryRun ? " [dry run]" : ""}`,
  );
  await runPush(oneoffContext, pushOptions);
  logFinishedStep(
    ctx,
    `${
      options.dryRun ? "Would have deployed" : "Deployed"
    } Convex functions to ${url}`,
  );
}

async function runCommand(
  ctx: Context,
  options: {
    cmdUrlEnvVarName?: string | undefined;
    cmd?: string | undefined;
    dryRun?: boolean | undefined;
    url: string;
  },
) {
  if (options.cmd === undefined) {
    return;
  }

  const urlVar =
    options.cmdUrlEnvVarName ?? (await suggestedEnvVarName(ctx)).envVar;
  showSpinner(
    ctx,
    `Running '${options.cmd}' with environment variable "${urlVar}" set...${
      options.dryRun ? " [dry run]" : ""
    }`,
  );
  if (!options.dryRun) {
    const env = { ...process.env };
    env[urlVar] = options.url;
    const result = spawnSync(options.cmd, {
      env,
      stdio: "inherit",
      shell: true,
    });
    if (result.status !== 0) {
      await ctx.crash({
        exitCode: 1,
        errorType: "invalid filesystem data",
        printedMessage: `'${options.cmd}' failed`,
      });
    }
  }
  logFinishedStep(
    ctx,
    `${options.dryRun ? "Would have run" : "Ran"} "${
      options.cmd
    }" with environment variable "${urlVar}" set`,
  );
}

async function askToConfirmPush(
  ctx: Context,
  deployment: {
    configuredName: string;
    configuredType: string | null;
    requestedName: string;
    requestedType: string;
  },
  prodUrl: string,
) {
  logMessage(
    ctx,
    `\
You're currently developing against your ${chalk.bold(
      deployment.configuredType ?? "dev",
    )} deployment

  ${deployment.configuredName} (set in CONVEX_DEPLOYMENT)

Your ${chalk.bold(deployment.requestedType)} deployment ${chalk.bold(
      deployment.requestedName,
    )} serves traffic at:

  ${(await suggestedEnvVarName(ctx)).envVar}=${chalk.bold(prodUrl)}

Make sure that your published client is configured with this URL (for instructions see https://docs.convex.dev/hosting)\n`,
  );
  return (
    await inquirer.prompt([
      {
        type: "confirm",
        name: "shouldPush",
        message: `Do you want to push your code to your ${deployment.requestedType} deployment ${deployment.requestedName} now?`,
        default: true,
      },
    ])
  ).shouldPush;
}
