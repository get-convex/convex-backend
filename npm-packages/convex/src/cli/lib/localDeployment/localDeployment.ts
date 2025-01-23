import { Context, logMessage, logVerbose } from "../../../bundler/context.js";
import detect from "detect-port";
import {
  bigBrainPause,
  bigBrainRecordActivity,
  bigBrainStart,
} from "./bigBrain.js";
import {
  LocalDeploymentConfig,
  loadDeploymentConfig,
  rootDeploymentStateDir,
  saveDeploymentConfig,
} from "./filePaths.js";
import {
  ensureBackendBinaryDownloaded,
  ensureBackendRunning,
  ensureBackendStopped,
  localDeploymentUrl,
  runLocalBackend,
} from "./run.js";
import { handlePotentialUpgrade } from "./upgrade.js";
import { OnDeploymentActivityFunc } from "../deployment.js";
import { promptSearch } from "../utils/prompts.js";
import { LocalDeploymentError, printLocalDeploymentOnError } from "./errors.js";
import chalk from "chalk";
export type DeploymentDetails = {
  deploymentName: string;
  deploymentUrl: string;
  adminKey: string;
  onActivity: OnDeploymentActivityFunc;
};

export async function handleLocalDeployment(
  ctx: Context,
  options: {
    teamSlug: string;
    projectSlug: string;
    ports?: {
      cloud: number;
      site: number;
    };
    backendVersion?: string;
    forceUpgrade: boolean;
  },
): Promise<DeploymentDetails> {
  if (await isOffline()) {
    return handleOffline(ctx, options);
  }

  const existingDeploymentForProject = await getExistingDeployment(ctx, {
    projectSlug: options.projectSlug,
    teamSlug: options.teamSlug,
  });
  if (existingDeploymentForProject === null) {
    printLocalDeploymentWelcomeMessage(ctx);
  }
  ctx.registerCleanup(async (_exitCode, err) => {
    if (err instanceof LocalDeploymentError) {
      printLocalDeploymentOnError(ctx);
    }
  });
  if (existingDeploymentForProject !== null) {
    logVerbose(
      ctx,
      `Found existing deployment for project ${options.projectSlug}`,
    );
    // If it's still running for some reason, exit and tell the user to kill it.
    // It's fine if a different backend is running on these ports though since we'll
    // pick new ones.
    await ensureBackendStopped(ctx, {
      ports: {
        cloud: existingDeploymentForProject.config.ports.cloud,
      },
      maxTimeSecs: 5,
      deploymentName: existingDeploymentForProject.deploymentName,
      allowOtherDeployments: true,
    });
  }

  const { binaryPath, version } = await ensureBackendBinaryDownloaded(
    ctx,
    options.backendVersion === undefined
      ? {
          kind: "latest",
        }
      : { kind: "version", version: options.backendVersion },
  );
  const ports = await choosePorts(ctx, options.ports);
  const { deploymentName, adminKey } = await bigBrainStart(ctx, {
    port: ports.cloud,
    projectSlug: options.projectSlug,
    teamSlug: options.teamSlug,
    instanceName: existingDeploymentForProject?.deploymentName ?? null,
  });
  const onActivity = async (isOffline: boolean, _wasOffline: boolean) => {
    await ensureBackendRunning(ctx, {
      cloudPort: ports.cloud,
      deploymentName,
      maxTimeSecs: 5,
    });
    if (isOffline) {
      return;
    }
    await bigBrainRecordActivity(ctx, {
      instanceName: deploymentName,
    });
  };

  const { cleanupHandle } = await handlePotentialUpgrade(ctx, {
    deploymentName,
    oldVersion: existingDeploymentForProject?.config.backendVersion ?? null,
    newBinaryPath: binaryPath,
    newVersion: version,
    ports,
    adminKey,
    forceUpgrade: options.forceUpgrade,
  });

  const cleanupFunc = ctx.removeCleanup(cleanupHandle);
  ctx.registerCleanup(async (exitCode, err) => {
    if (cleanupFunc !== null) {
      await cleanupFunc(exitCode, err);
    }
    await bigBrainPause(ctx, {
      projectSlug: options.projectSlug,
      teamSlug: options.teamSlug,
    });
  });

  return {
    adminKey,
    deploymentName,
    deploymentUrl: localDeploymentUrl(ports.cloud),
    onActivity,
  };
}

async function handleOffline(
  ctx: Context,
  options: {
    teamSlug: string;
    projectSlug: string;
    ports?: { cloud: number; site: number };
  },
): Promise<DeploymentDetails> {
  const { deploymentName, config } =
    await chooseFromExistingLocalDeployments(ctx);
  const { binaryPath } = await ensureBackendBinaryDownloaded(ctx, {
    kind: "version",
    version: config.backendVersion,
  });
  const ports = await choosePorts(ctx, options.ports);
  saveDeploymentConfig(ctx, deploymentName, config);
  await runLocalBackend(ctx, {
    binaryPath,
    ports,
    deploymentName,
  });
  return {
    adminKey: config.adminKey,
    deploymentName,
    deploymentUrl: localDeploymentUrl(ports.cloud),
    onActivity: async (isOffline: boolean, wasOffline: boolean) => {
      await ensureBackendRunning(ctx, {
        cloudPort: ports.cloud,
        deploymentName,
        maxTimeSecs: 5,
      });
      if (isOffline) {
        return;
      }
      if (wasOffline) {
        await bigBrainStart(ctx, {
          port: ports.cloud,
          projectSlug: options.projectSlug,
          teamSlug: options.teamSlug,
          instanceName: deploymentName,
        });
      }
      await bigBrainRecordActivity(ctx, {
        instanceName: deploymentName,
      });
    },
  };
}

async function getExistingDeployment(
  ctx: Context,
  options: {
    projectSlug: string;
    teamSlug: string;
  },
): Promise<{ deploymentName: string; config: LocalDeploymentConfig } | null> {
  const { projectSlug, teamSlug } = options;
  const prefix = `local-${teamSlug.replace(/-/g, "_")}-${projectSlug.replace(/-/g, "_")}`;
  const localDeployments = await getLocalDeployments(ctx);
  const existingDeploymentForProject = localDeployments.find((d) =>
    d.deploymentName.startsWith(prefix),
  );
  if (existingDeploymentForProject === undefined) {
    return null;
  }
  return {
    deploymentName: existingDeploymentForProject.deploymentName,
    config: existingDeploymentForProject.config,
  };
}

async function getLocalDeployments(ctx: Context): Promise<
  Array<{
    deploymentName: string;
    config: LocalDeploymentConfig;
  }>
> {
  const dir = rootDeploymentStateDir();
  if (!ctx.fs.exists(dir)) {
    return [];
  }
  const deploymentNames = ctx.fs.listDir(dir).map((d) => d.name);
  return deploymentNames.flatMap((deploymentName) => {
    const config = loadDeploymentConfig(ctx, deploymentName);
    if (config !== null) {
      return [{ deploymentName, config }];
    }
    return [];
  });
}

async function chooseFromExistingLocalDeployments(ctx: Context): Promise<{
  deploymentName: string;
  config: LocalDeploymentConfig;
}> {
  const localDeployments = await getLocalDeployments(ctx);
  return promptSearch(ctx, {
    message: "Choose from an existing local deployment?",
    choices: localDeployments.map((d) => ({
      name: d.deploymentName,
      value: d,
    })),
  });
}

async function choosePorts(
  ctx: Context,
  requestedPorts?: {
    cloud: number;
    site: number;
  },
): Promise<{ cloud: number; site: number }> {
  if (requestedPorts !== undefined) {
    const availableCloudPort = await detect(requestedPorts.cloud);
    const availableSitePort = await detect(requestedPorts.site);
    if (
      availableCloudPort !== requestedPorts.cloud ||
      availableSitePort !== requestedPorts.site
    ) {
      return ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: "Requested ports are not available",
      });
    }
    return { cloud: availableCloudPort, site: availableSitePort };
  }
  const availableCloudPort = await detect(3210);
  const availableSitePort = await detect(availableCloudPort + 1);
  return { cloud: availableCloudPort, site: availableSitePort };
}

async function isOffline(): Promise<boolean> {
  // TODO(ENG-7080) -- implement this for real
  return false;
}

function printLocalDeploymentWelcomeMessage(ctx: Context) {
  logMessage(
    ctx,
    chalk.cyan("You're trying out the beta local deployment feature!"),
  );
  logMessage(
    ctx,
    chalk.cyan("To learn more, read the docs: https://cvx.so/local-deployment"),
  );
  logMessage(
    ctx,
    chalk.cyan("To opt out at any time, remove `--local` from your command."),
  );
}
