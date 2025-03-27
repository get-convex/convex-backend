// ----------------------------------------------------------------------------
// Try it out (No account)

import path from "path";
import {
  Context,
  logFinishedStep,
  logMessage,
  logVerbose,
  logWarning,
} from "../../../bundler/context.js";
import { promptSearch, promptString, promptYesNo } from "../utils/prompts.js";
import {
  bigBrainGenerateTryItOutAdminKey,
  bigBrainPause,
  bigBrainStart,
} from "./bigBrain.js";
import { LocalDeploymentError, printLocalDeploymentOnError } from "./errors.js";
import {
  LocalDeploymentKind,
  deploymentStateDir,
  loadDeploymentConfig,
  saveDeploymentConfig,
} from "./filePaths.js";
import { rootDeploymentStateDir } from "./filePaths.js";
import { LocalDeploymentConfig } from "./filePaths.js";
import { DeploymentDetails } from "./localDeployment.js";
import { ensureBackendStopped, localDeploymentUrl } from "./run.js";
import { ensureBackendRunning } from "./run.js";
import { handlePotentialUpgrade } from "./upgrade.js";
import {
  isOffline,
  generateInstanceSecret,
  choosePorts,
  LOCAL_BACKEND_INSTANCE_SECRET,
} from "./utils.js";
import { handleDashboard } from "./dashboard.js";
import crypto from "crypto";
import { recursivelyDelete, recursivelyCopy } from "../fsUtils.js";
import { ensureBackendBinaryDownloaded } from "./download.js";
import { isTryItOutDeployment } from "../deployment.js";
import { createProject } from "../api.js";
import { removeTryItOutPrefix } from "../deployment.js";
import { nodeFs } from "../../../bundler/fs.js";

export async function handleTryItOutDeployment(
  ctx: Context,
  options: {
    ports?: {
      cloud: number;
      site: number;
    };
    backendVersion?: string;
    dashboardVersion?: string;
    forceUpgrade: boolean;
    deploymentName: string | null;
    chosenConfiguration: "new" | "existing" | "ask" | null;
  },
): Promise<DeploymentDetails> {
  if (await isOffline()) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: "Cannot run a try-it-out deployment in offline mode",
    });
  }

  const deployment = await chooseTryItOutDeployment(ctx, {
    deploymentName: options.deploymentName,
    chosenConfiguration: options.chosenConfiguration,
  });
  if (deployment.kind === "first") {
    logMessage(
      ctx,
      "This command, `npx convex dev`, will run your deployment and update it with the function you write in the `convex/` directory.",
    );
    logMessage(
      ctx,
      "Use `npx convex dashboard` to view and interact with your deployment from a web UI.",
    );
    logMessage(
      ctx,
      "Use `npx convex docs` to read the docs and `npx convex help` to see other commands.",
    );
    if (process.stdin.isTTY) {
      const result = await promptYesNo(ctx, {
        message: "Got it? Let's get started!",
        default: true,
      });
      if (!result) {
        return ctx.crash({
          exitCode: 1,
          errorType: "fatal",
          printedMessage: "Exiting",
        });
      }
    }
  }
  ctx.registerCleanup(async (_exitCode, err) => {
    if (err instanceof LocalDeploymentError) {
      printLocalDeploymentOnError(ctx);
    }
  });
  const { binaryPath, version } = await ensureBackendBinaryDownloaded(
    ctx,
    options.backendVersion === undefined
      ? {
          kind: "latest",
        }
      : { kind: "version", version: options.backendVersion },
  );
  await handleDashboard(ctx, version);
  let adminKey: string;
  let instanceSecret: string;
  if (deployment.kind === "existing") {
    adminKey = deployment.config.adminKey;
    instanceSecret =
      deployment.config.instanceSecret ?? LOCAL_BACKEND_INSTANCE_SECRET;
    // If it's still running for some reason, exit and tell the user to kill it.
    // It's fine if a different backend is running on these ports though since we'll
    // pick new ones.
    await ensureBackendStopped(ctx, {
      ports: {
        cloud: deployment.config.ports.cloud,
      },
      maxTimeSecs: 5,
      deploymentName: deployment.deploymentName,
      allowOtherDeployments: true,
    });
  } else {
    instanceSecret = generateInstanceSecret();
    const data = await bigBrainGenerateTryItOutAdminKey(ctx, {
      instanceName: deployment.deploymentName,
      instanceSecret,
    });
    adminKey = data.adminKey;
  }

  const [cloudPort, sitePort] = await choosePorts(ctx, {
    count: 2,
    startPort: 3210,
    requestedPorts: [options.ports?.cloud ?? null, options.ports?.site ?? null],
  });
  const onActivity = async (isOffline: boolean, _wasOffline: boolean) => {
    await ensureBackendRunning(ctx, {
      cloudPort,
      deploymentName: deployment.deploymentName,
      maxTimeSecs: 5,
    });
    if (isOffline) {
      return;
    }
  };

  const { cleanupHandle } = await handlePotentialUpgrade(ctx, {
    deploymentName: deployment.deploymentName,
    deploymentKind: "tryItOut",
    oldVersion:
      deployment.kind === "existing" ? deployment.config.backendVersion : null,
    newBinaryPath: binaryPath,
    newVersion: version,
    ports: { cloud: cloudPort, site: sitePort },
    adminKey,
    instanceSecret,
    forceUpgrade: options.forceUpgrade,
  });

  const cleanupFunc = ctx.removeCleanup(cleanupHandle);
  ctx.registerCleanup(async (exitCode, err) => {
    if (cleanupFunc !== null) {
      await cleanupFunc(exitCode, err);
    }
  });

  return {
    adminKey,
    deploymentName: deployment.deploymentName,
    deploymentUrl: localDeploymentUrl(cloudPort),
    onActivity,
  };
}

export async function loadTryItOutDeployment(
  ctx: Context,
  deploymentName: string,
): Promise<LocalDeploymentConfig> {
  const config = loadDeploymentConfig(ctx, "tryItOut", deploymentName);
  if (config === null) {
    return ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `Could not find deployment with name ${deploymentName}!`,
    });
  }
  return config;
}

export async function listExistingTryItOutDeployments(ctx: Context): Promise<
  Array<{
    deploymentName: string;
    config: LocalDeploymentConfig;
  }>
> {
  const dir = rootDeploymentStateDir("tryItOut");
  if (!ctx.fs.exists(dir)) {
    return [];
  }
  const deploymentNames = ctx.fs
    .listDir(dir)
    .map((d) => d.name)
    .filter((d) => isTryItOutDeployment(d));
  return deploymentNames.flatMap((deploymentName) => {
    const config = loadDeploymentConfig(ctx, "tryItOut", deploymentName);
    if (config !== null) {
      return [{ deploymentName, config }];
    }
    return [];
  });
}

async function chooseTryItOutDeployment(
  ctx: Context,
  options: {
    deploymentName: string | null;
    chosenConfiguration: "new" | "existing" | "ask" | null;
  },
): Promise<
  | {
      kind: "existing";
      deploymentName: string;
      config: LocalDeploymentConfig;
    }
  | {
      kind: "new";
      deploymentName: string;
    }
  | {
      kind: "first";
      deploymentName: string;
    }
> {
  const deployments = await listExistingTryItOutDeployments(ctx);
  if (options.deploymentName !== null && options.chosenConfiguration === null) {
    const existing = deployments.find(
      (d) => d.deploymentName === options.deploymentName,
    );
    if (existing === undefined) {
      logWarning(
        ctx,
        `Could not find deployment with name ${options.deploymentName}!`,
      );
    } else {
      return {
        kind: "existing",
        deploymentName: existing.deploymentName,
        config: existing.config,
      };
    }
  }
  if (deployments.length === 0) {
    logMessage(
      ctx,
      "Welcome to developing with Convex. Let's set up your first deployment.",
    );
    return await promptForNewDeployment(ctx, []);
  }

  if (options.chosenConfiguration === "new") {
    const deploymentName = await promptString(ctx, {
      message: "Choose a name for your new deployment:",
      default: path.basename(process.cwd()),
    });
    const uniqueName = await getUniqueName(
      ctx,
      deploymentName,
      deployments.map((d) => d.deploymentName),
    );
    logVerbose(ctx, `Deployment name: ${uniqueName}`);
    return {
      kind: "new",
      deploymentName: uniqueName,
    };
  }

  const newOrExisting = await promptSearch(ctx, {
    message: "Which deployment would you like to use?",
    choices: [
      ...(options.chosenConfiguration === "existing"
        ? []
        : [
            {
              name: "Create a new one",
              value: "new",
            },
          ]),
      ...deployments.map((d) => ({
        name: d.deploymentName,
        value: d.deploymentName,
      })),
    ],
  });

  if (newOrExisting !== "new") {
    const existingDeployment = deployments.find(
      (d) => d.deploymentName === newOrExisting,
    );
    if (existingDeployment === undefined) {
      return ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: `Could not find deployment with name ${newOrExisting}!`,
      });
    }
    return {
      kind: "existing",
      deploymentName: existingDeployment.deploymentName,
      config: existingDeployment.config,
    };
  }
  return await promptForNewDeployment(
    ctx,
    deployments.map((d) => d.deploymentName),
  );
}

async function promptForNewDeployment(
  ctx: Context,
  existingNames: string[],
): Promise<
  | {
      kind: "first";
      deploymentName: string;
    }
  | {
      kind: "new";
      deploymentName: string;
    }
> {
  const isFirstDeployment = existingNames.length === 0;
  const message = isFirstDeployment
    ? "Choose a name for your first deployment:"
    : "Choose a name:";
  const deploymentName = await promptString(ctx, {
    message,
    default: path.basename(process.cwd()),
  });

  const uniqueName = await getUniqueName(
    ctx,
    `tryitout-${deploymentName}`,
    existingNames,
  );
  logVerbose(ctx, `Deployment name: ${uniqueName}`);
  return isFirstDeployment
    ? {
        kind: "first",
        deploymentName: uniqueName,
      }
    : {
        kind: "new",
        deploymentName: uniqueName,
      };
}

async function getUniqueName(
  ctx: Context,
  name: string,
  existingNames: string[],
) {
  if (!existingNames.includes(name)) {
    return name;
  }
  for (let i = 1; i <= 5; i++) {
    const uniqueName = `${name}-${i}`;
    if (!existingNames.includes(uniqueName)) {
      return uniqueName;
    }
  }
  const randomSuffix = crypto.randomBytes(4).toString("hex");

  const uniqueName = `${name}-${randomSuffix}`;
  if (!existingNames.includes(uniqueName)) {
    return uniqueName;
  }
  return ctx.crash({
    exitCode: 1,
    errorType: "fatal",
    printedMessage: `Could not generate a unique name for your deployment, please choose a different name`,
  });
}
/**
 * This takes a "try it out" deployment and makes it a "local" deployment
 * that is associated with a project in the given team.
 */
export async function handleLinkToProject(
  ctx: Context,
  args: {
    deploymentName: string;
    teamSlug: string;
    projectSlug: string | null;
  },
): Promise<{
  deploymentName: string;
  deploymentUrl: string;
  projectSlug: string;
}> {
  logVerbose(
    ctx,
    `Linking ${args.deploymentName} to a project in team ${args.teamSlug}`,
  );
  const config = await loadDeploymentConfig(
    ctx,
    "tryItOut",
    args.deploymentName,
  );
  if (config === null) {
    return ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: "Failed to load deployment config",
    });
  }
  await ensureBackendStopped(ctx, {
    ports: {
      cloud: config.ports.cloud,
    },
    deploymentName: args.deploymentName,
    allowOtherDeployments: true,
    maxTimeSecs: 5,
  });
  const projectName = removeTryItOutPrefix(args.deploymentName);
  let projectSlug: string;
  if (args.projectSlug !== null) {
    projectSlug = args.projectSlug;
  } else {
    const { projectSlug: newProjectSlug } = await createProject(ctx, {
      teamSlug: args.teamSlug,
      projectName,
      deploymentTypeToProvision: "prod",
    });
    projectSlug = newProjectSlug;
  }
  logVerbose(ctx, `Creating local deployment in project ${projectSlug}`);
  // Register it in big brain
  const { deploymentName: localDeploymentName, adminKey } = await bigBrainStart(
    ctx,
    {
      port: config.ports.cloud,
      projectSlug,
      teamSlug: args.teamSlug,
      instanceName: null,
    },
  );
  const localConfig = loadDeploymentConfig(ctx, "local", localDeploymentName);
  if (localConfig !== null) {
    return ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `Project ${projectSlug} already has a local deployment, so we cannot link this try-it-out deployment to it.`,
    });
  }
  logVerbose(ctx, `Moving ${args.deploymentName} to ${localDeploymentName}`);
  await moveDeployment(
    ctx,
    {
      deploymentKind: "tryItOut",
      deploymentName: args.deploymentName,
    },
    {
      deploymentKind: "local",
      deploymentName: localDeploymentName,
    },
  );
  logVerbose(ctx, `Saving deployment config for ${localDeploymentName}`);
  await saveDeploymentConfig(ctx, "local", localDeploymentName, {
    adminKey,
    backendVersion: config.backendVersion,
    ports: config.ports,
  });
  await bigBrainPause(ctx, {
    projectSlug,
    teamSlug: args.teamSlug,
  });
  logFinishedStep(
    ctx,
    `Linked ${args.deploymentName} to project ${projectSlug}`,
  );
  return {
    projectSlug,
    deploymentName: localDeploymentName,
    deploymentUrl: localDeploymentUrl(config.ports.cloud),
  };
}

export async function moveDeployment(
  ctx: Context,
  oldDeployment: {
    deploymentKind: LocalDeploymentKind;
    deploymentName: string;
  },
  newDeployment: {
    deploymentKind: LocalDeploymentKind;
    deploymentName: string;
  },
) {
  const oldPath = deploymentStateDir(
    oldDeployment.deploymentKind,
    oldDeployment.deploymentName,
  );
  const newPath = deploymentStateDir(
    newDeployment.deploymentKind,
    newDeployment.deploymentName,
  );
  await recursivelyCopy(ctx, nodeFs, oldPath, newPath);
  recursivelyDelete(ctx, oldPath);
}
