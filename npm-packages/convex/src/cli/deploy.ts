import chalk from "chalk";
import { Command, Option } from "@commander-js/extra-typings";
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
  CONVEX_SELF_HOSTED_URL_VAR_NAME,
  bigBrainAPI,
  getConfiguredDeployment,
  readAdminKeyFromEnvVar,
} from "./lib/utils/utils.js";
import { runFunctionAndLog } from "./lib/run.js";
import { usageStateWarning } from "./lib/usage.js";
import {
  CONVEX_DEPLOYMENT_VAR_NAME,
  deploymentTypeFromAdminKey,
  getConfiguredDeploymentFromEnvVar,
  isPreviewDeployKey,
} from "./lib/deployment.js";
import { runPush } from "./lib/components.js";
import { promptYesNo } from "./lib/utils/prompts.js";
import { deployToDeployment, runCommand } from "./lib/deploy2.js";

export const deploy = new Command("deploy")
  .summary("Deploy to your prod deployment")
  .description(
    "Deploy to your deployment. By default, this deploys to your prod deployment.\n\n" +
      "Deploys to a preview deployment if the `CONVEX_DEPLOY_KEY` environment variable is set to a Preview Deploy Key.",
  )
  .allowExcessArguments(false)
  .addDeployOptions()
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
  // Hidden options to pass in admin key and url for tests and local development
  .addOption(new Option("--admin-key <adminKey>").hideHelp())
  .addOption(new Option("--url <url>").hideHelp())
  .addOption(
    new Option(
      "--preview-name <name>",
      "[deprecated] Use `--preview-create` instead. The name to associate with this deployment if deploying to a preview deployment.",
    )
      .hideHelp()
      .conflicts("preview-create"),
  )
  .addOption(
    new Option(
      "--env-file <envFile>",
      `Path to a custom file of environment variables, for choosing the \
deployment, e.g. ${CONVEX_DEPLOYMENT_VAR_NAME} or ${CONVEX_SELF_HOSTED_URL_VAR_NAME}. \
Same format as .env.local or .env files, and overrides them.`,
    ),
  )
  .addOption(new Option("--partition-id <id>").hideHelp())
  .showHelpAfterError()
  .action(async (cmdOptions) => {
    const ctx = oneoffContext();

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

    if (
      configuredDeployKey !== null &&
      isPreviewDeployKey(configuredDeployKey)
    ) {
      await usageStateWarning(ctx);
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
    typecheckComponents: boolean;
    codegen: "enable" | "disable";

    debug?: boolean | undefined;
    debugBundlePath?: string | undefined;
    partitionId?: string | undefined;
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
        (await getConfiguredDeployment(ctx)).name,
        options.configuredDeployKey,
      ),
      identifier: previewName,
      partitionId: options.partitionId
        ? parseInt(options.partitionId)
        : undefined,
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
    typecheckComponents: options.typecheckComponents,
    debug: !!options.debug,
    debugBundlePath: options.debugBundlePath,
    codegen: options.codegen === "enable",
    url: previewUrl,
    liveComponentSources: false,
  };
  showSpinner(ctx, `Deploying to ${previewUrl}...`);
  await runPush(ctx, pushOptions);
  logFinishedStep(ctx, `Deployed Convex functions to ${previewUrl}`);

  if (options.previewRun !== undefined) {
    await runFunctionAndLog(ctx, {
      deploymentUrl: previewUrl,
      adminKey: previewAdminKey,
      functionName: options.previewRun,
      argsString: "{}",
      componentPath: undefined,
      callbacks: {
        onSuccess: () => {
          logFinishedStep(
            ctx,
            `Finished running function "${options.previewRun}"`,
          );
        },
      },
    });
  }
}

async function deployToExistingDeployment(
  ctx: Context,
  options: {
    verbose?: boolean | undefined;
    dryRun?: boolean | undefined;
    yes?: boolean | undefined;
    typecheck: "enable" | "try" | "disable";
    typecheckComponents: boolean;
    codegen: "enable" | "disable";
    cmd?: string | undefined;
    cmdUrlEnvVarName?: string | undefined;

    debugBundlePath?: string | undefined;
    debug?: boolean | undefined;
    adminKey?: string | undefined;
    url?: string | undefined;
    writePushRequest?: string | undefined;
    liveComponentSources?: boolean | undefined;
    partitionId?: string | undefined;
    envFile?: string | undefined;
  },
) {
  const deploymentSelection = await deploymentSelectionFromOptions(ctx, {
    ...options,
    implicitProd: true,
  });
  if (deploymentSelection.kind !== "urlWithAdminKey") {
    await usageStateWarning(ctx);
  }
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

  await deployToDeployment(ctx, { url, adminKey }, options);
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
  return promptYesNo(ctx, {
    message: `Do you want to push your code to your ${deployment.requestedType} deployment ${deployment.requestedName} now?`,
    default: true,
  });
}
