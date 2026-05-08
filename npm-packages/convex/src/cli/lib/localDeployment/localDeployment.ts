import { Context } from "../../../bundler/context.js";
import {
  logFinishedStep,
  logVerbose,
  showSpinner,
} from "../../../bundler/log.js";
import {
  bigBrainPause,
  bigBrainRecordActivity,
  bigBrainStart,
} from "./bigBrain.js";
import {
  LocalDeploymentConfig,
  loadDeploymentConfig,
  loadDeploymentConfigFromDir,
  loadProjectLocalConfig,
  legacyDeploymentStateDir,
  rootDeploymentStateDir,
  saveDeploymentConfig,
} from "./filePaths.js";
import {
  ensureBackendRunning,
  ensureBackendStopped,
  localDeploymentUrl,
  runLocalBackend,
  withRunningBackend,
} from "./run.js";
import { handlePotentialUpgrade } from "./upgrade.js";
import { OnDeploymentActivityFunc } from "../deployment.js";
import { promptSearch } from "../utils/prompts.js";
import { LocalDeploymentError, printLocalDeploymentOnError } from "./errors.js";
import {
  chooseLocalBackendPorts,
  printLocalDeploymentWelcomeMessage,
  isOffline,
  LOCAL_BACKEND_INSTANCE_SECRET,
} from "./utils.js";
import { ensureBackendBinaryDownloaded } from "./download.js";
import { defaultEnvBackend } from "../defaultEnv.js";
import { deploymentEnvBackend } from "../env.js";
import { getProjectDetails } from "../deploymentSelection.js";

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
    ports: {
      cloud: number | undefined;
      site: number | undefined;
    };
    backendVersion?: string | undefined;
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
  const isFirstTime = existingDeploymentForProject === null;
  if (isFirstTime) {
    printLocalDeploymentWelcomeMessage();
  }
  ctx.registerCleanup(async (_exitCode, err) => {
    if (err instanceof LocalDeploymentError) {
      printLocalDeploymentOnError();
    }
  });
  if (existingDeploymentForProject !== null) {
    logVerbose(`Found existing deployment for project ${options.projectSlug}`);
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
          allowedVersion: existingDeploymentForProject?.config.backendVersion,
        }
      : { kind: "version", version: options.backendVersion },
  );
  const { cloudPort, sitePort } = await chooseLocalBackendPorts(ctx, {
    requestedPorts: options.ports,
    suggestedPorts: existingDeploymentForProject?.config.ports,
  });
  const { deploymentName, adminKey } = await bigBrainStart(ctx, {
    port: cloudPort,
    projectSlug: options.projectSlug,
    teamSlug: options.teamSlug,
    instanceName: existingDeploymentForProject?.deploymentName ?? null,
  });
  const onActivity = async (isOffline: boolean, _wasOffline: boolean) => {
    await ensureBackendRunning(ctx, {
      cloudPort,
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
    deploymentKind: "local",
    deploymentName,
    oldVersion: existingDeploymentForProject?.config.backendVersion ?? null,
    newBinaryPath: binaryPath,
    newVersion: version,
    ports: { cloud: cloudPort, site: sitePort },
    adminKey,
    instanceSecret: LOCAL_BACKEND_INSTANCE_SECRET,
    forceUpgrade: options.forceUpgrade,
  });

  if (isFirstTime) {
    await importDefaultEnvVars(ctx, {
      teamSlug: options.teamSlug,
      projectSlug: options.projectSlug,
      deploymentName,
      deploymentUrl: localDeploymentUrl(cloudPort),
      adminKey,
    });
  }

  // Periodically report activity to BigBrain every 60 seconds.
  // Uses self-scheduling setTimeout to avoid overlapping requests.
  let activityTimeout: ReturnType<typeof setTimeout> | null = null;
  const scheduleActivityPing = () => {
    activityTimeout = setTimeout(async () => {
      try {
        await bigBrainRecordActivity(ctx, {
          instanceName: deploymentName,
        });
      } catch {
        // Best-effort: don't crash on failed pings
      }
      scheduleActivityPing();
    }, 60_000);
  };
  scheduleActivityPing();

  const cleanupFunc = ctx.removeCleanup(cleanupHandle);
  ctx.registerCleanup(async (exitCode, err) => {
    if (activityTimeout !== null) {
      clearTimeout(activityTimeout);
    }
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
    deploymentUrl: localDeploymentUrl(cloudPort),
    onActivity,
  };
}

export async function loadLocalDeploymentCredentials(
  ctx: Context,
  deploymentName: string,
): Promise<{
  deploymentName: string;
  deploymentUrl: string;
  adminKey: string;
}> {
  const config = loadDeploymentConfig(ctx, "local", deploymentName);
  if (config === null) {
    return ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage:
        "Failed to load deployment config - try running `npx convex dev --configure`",
    });
  }
  return {
    deploymentName,
    deploymentUrl: localDeploymentUrl(config.ports.cloud),
    adminKey: config.adminKey,
  };
}

async function handleOffline(
  ctx: Context,
  options: {
    teamSlug: string;
    projectSlug: string;
    ports: { cloud: number | undefined; site: number | undefined };
  },
): Promise<DeploymentDetails> {
  const { deploymentName, config } =
    await chooseFromExistingLocalDeployments(ctx);
  const { binaryPath } = await ensureBackendBinaryDownloaded(ctx, {
    kind: "version",
    version: config.backendVersion,
  });
  const { cloudPort, sitePort } = await chooseLocalBackendPorts(ctx, {
    requestedPorts: options.ports,
    // FIXME: This doesn’t try to reuse the ports already assigned in the config.
    // Please update this if we ever support offline mode (currently this is dead code).
  });
  saveDeploymentConfig(ctx, "local", deploymentName, config);
  await runLocalBackend(ctx, {
    binaryPath,
    ports: { cloud: cloudPort, site: sitePort },
    deploymentName,
    deploymentKind: "local",
    instanceSecret: LOCAL_BACKEND_INSTANCE_SECRET,
    isLatestVersion: false,
  });
  return {
    adminKey: config.adminKey,
    deploymentName,
    deploymentUrl: localDeploymentUrl(cloudPort),
    onActivity: async (isOffline: boolean, wasOffline: boolean) => {
      await ensureBackendRunning(ctx, {
        cloudPort,
        deploymentName,
        maxTimeSecs: 5,
      });
      if (isOffline) {
        return;
      }
      if (wasOffline) {
        await bigBrainStart(ctx, {
          port: cloudPort,
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

  // Check project-local storage first - this is the new default location
  const projectLocal = loadProjectLocalConfig(ctx);
  if (projectLocal !== null) {
    // Verify this deployment is for the expected project (matches the naming pattern)
    const expectedPrefix = `local-${teamSlug.replace(/-/g, "_")}-${projectSlug.replace(/-/g, "_")}`;
    if (projectLocal.deploymentName.startsWith(expectedPrefix)) {
      return projectLocal;
    }
    logVerbose(
      `Project-local deployment ${projectLocal.deploymentName} doesn't match expected prefix ${expectedPrefix}`,
    );
  }

  // Fall back to checking legacy home directory
  const prefix = `local-${teamSlug.replace(/-/g, "_")}-${projectSlug.replace(/-/g, "_")}`;
  const legacyDeployments = await getLegacyLocalDeployments(ctx);
  const existingDeploymentForProject = legacyDeployments.find((d) =>
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

/**
 * Get local deployments from the legacy home directory location.
 * This is used for backward compatibility and for listing deployments in offline mode.
 */
async function getLegacyLocalDeployments(ctx: Context): Promise<
  Array<{
    deploymentName: string;
    config: LocalDeploymentConfig;
  }>
> {
  const dir = rootDeploymentStateDir("local");
  if (!ctx.fs.exists(dir)) {
    return [];
  }
  const deploymentNames = ctx.fs
    .listDir(dir)
    .map((d) => d.name)
    .filter((d) => d.startsWith("local-"));
  return deploymentNames.flatMap((deploymentName) => {
    const legacyDir = legacyDeploymentStateDir("local", deploymentName);
    const config = loadDeploymentConfigFromDir(ctx, legacyDir);
    if (config !== null) {
      return [{ deploymentName, config }];
    }
    return [];
  });
}

/**
 * Get all local deployments from both project-local and legacy locations.
 */
async function getLocalDeployments(ctx: Context): Promise<
  Array<{
    deploymentName: string;
    config: LocalDeploymentConfig;
  }>
> {
  const deployments: Array<{
    deploymentName: string;
    config: LocalDeploymentConfig;
  }> = [];

  // Check project-local storage
  const projectLocal = loadProjectLocalConfig(ctx);
  if (
    projectLocal !== null &&
    projectLocal.deploymentName.startsWith("local-")
  ) {
    deployments.push(projectLocal);
  }

  // Also include legacy deployments (but avoid duplicates)
  const legacyDeployments = await getLegacyLocalDeployments(ctx);
  for (const legacy of legacyDeployments) {
    if (!deployments.some((d) => d.deploymentName === legacy.deploymentName)) {
      deployments.push(legacy);
    }
  }

  return deployments;
}

async function chooseFromExistingLocalDeployments(ctx: Context): Promise<{
  deploymentName: string;
  config: LocalDeploymentConfig;
}> {
  const localDeployments = await getLocalDeployments(ctx);

  if (localDeployments.length === 0) {
    return ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage:
        "No local deployments found. Please run `npx convex dev` while online first.",
    });
  }

  // Auto-select if there's only one deployment
  if (localDeployments.length === 1) {
    logVerbose(
      `Auto-selecting the only local deployment: ${localDeployments[0].deploymentName}`,
    );
    return localDeployments[0];
  }

  // Multiple deployments (legacy) - prompt user to choose
  return promptSearch(ctx, {
    message: "Choose from an existing local deployment:",
    choices: localDeployments.map((d) => ({
      name: d.deploymentName,
      value: d,
    })),
  });
}

/** Copies the default dev env vars from big brain the first time the local dev backend is started */
export async function importDefaultEnvVars(
  ctx: Context,
  {
    teamSlug,
    projectSlug,
    deploymentName,
    deploymentUrl,
    adminKey,
  }: {
    teamSlug: string;
    projectSlug: string;
    deploymentName: string;
    deploymentUrl: string;
    adminKey: string;
  },
) {
  showSpinner("Importing default env vars...");

  const project = await getProjectDetails(ctx, {
    kind: "teamAndProjectSlugs",
    teamSlug,
    projectSlug,
  });
  const defaults = await defaultEnvBackend(ctx, project.id, "dev").list();
  if (defaults.length === 0) {
    logFinishedStep("No default env vars to import.");
    return;
  }

  const deployment = {
    deploymentUrl,
    deploymentFields: {
      deploymentName,
      deploymentType: "local" as const,
      projectSlug,
      teamSlug,
    },
  };

  await withRunningBackend({
    ctx,
    deployment,
    action: async () => {
      await deploymentEnvBackend(ctx, { deploymentUrl, adminKey }).update(
        defaults.map((v) => ({ name: v.name, value: v.value })),
      );
      logFinishedStep(
        `Imported ${defaults.length} environment ${defaults.length === 1 ? "variable" : "variables"} from default environment variables: ${defaults.map((v) => v.name).join(", ")}`,
      );
    },
  });
}
