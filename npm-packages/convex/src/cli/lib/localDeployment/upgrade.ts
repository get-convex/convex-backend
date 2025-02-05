import path from "path";
import {
  Context,
  logFailure,
  logFinishedStep,
  logVerbose,
} from "../../../bundler/context.js";
import { runSystemQuery } from "../run.js";
import { deploymentStateDir, saveDeploymentConfig } from "./filePaths.js";
import {
  ensureBackendBinaryDownloaded,
  ensureBackendStopped,
  localDeploymentUrl,
  runLocalBackend,
} from "./run.js";
import {
  downloadSnapshotExport,
  startSnapshotExport,
} from "../convexExport.js";
import { deploymentFetch, logAndHandleFetchError } from "../utils/utils.js";
import {
  confirmImport,
  uploadForImport,
  waitForStableImportState,
} from "../convexImport.js";
import { promptOptions, promptYesNo } from "../utils/prompts.js";
import { recursivelyDelete } from "../fsUtils.js";
import { LocalDeploymentError } from "./errors.js";

export async function handlePotentialUpgrade(
  ctx: Context,
  args: {
    deploymentName: string;
    oldVersion: string | null;
    newBinaryPath: string;
    newVersion: string;
    ports: {
      cloud: number;
      site: number;
    };
    adminKey: string;
    forceUpgrade: boolean;
  },
): Promise<{ cleanupHandle: string }> {
  const newConfig = {
    ports: args.ports,
    backendVersion: args.newVersion,
    adminKey: args.adminKey,
  };
  if (args.oldVersion === null || args.oldVersion === args.newVersion) {
    // No upgrade needed. Save the current config and start running the backend.
    saveDeploymentConfig(ctx, args.deploymentName, newConfig);
    return runLocalBackend(ctx, {
      binaryPath: args.newBinaryPath,
      deploymentName: args.deploymentName,
      ports: args.ports,
    });
  }
  logVerbose(
    ctx,
    `Considering upgrade from ${args.oldVersion} to ${args.newVersion}`,
  );
  const confirmed =
    args.forceUpgrade ||
    (await promptYesNo(ctx, {
      message: `This deployment is using an older version of the Convex backend. Upgrade now?`,
      default: true,
    }));
  if (!confirmed) {
    const { binaryPath: oldBinaryPath } = await ensureBackendBinaryDownloaded(
      ctx,
      {
        kind: "version",
        version: args.oldVersion,
      },
    );
    // Skipping upgrade, save the config with the old version and run.
    saveDeploymentConfig(ctx, args.deploymentName, {
      ...newConfig,
      backendVersion: args.oldVersion,
    });
    return runLocalBackend(ctx, {
      binaryPath: oldBinaryPath,
      ports: args.ports,
      deploymentName: args.deploymentName,
    });
  }
  const choice = args.forceUpgrade
    ? "transfer"
    : await promptOptions(ctx, {
        message: "Transfer data from existing deployment?",
        default: "transfer",
        choices: [
          { name: "transfer data", value: "transfer" },
          { name: "start fresh", value: "reset" },
        ],
      });
  const deploymentStatePath = deploymentStateDir(args.deploymentName);
  if (choice === "reset") {
    recursivelyDelete(ctx, deploymentStatePath, { force: true });
    saveDeploymentConfig(ctx, args.deploymentName, newConfig);
    return runLocalBackend(ctx, {
      binaryPath: args.newBinaryPath,
      deploymentName: args.deploymentName,
      ports: args.ports,
    });
  }
  return handleUpgrade(ctx, {
    deploymentName: args.deploymentName,
    oldVersion: args.oldVersion!,
    newBinaryPath: args.newBinaryPath,
    newVersion: args.newVersion,
    ports: args.ports,
    adminKey: args.adminKey,
  });
}

async function handleUpgrade(
  ctx: Context,
  args: {
    deploymentName: string;
    oldVersion: string;
    newBinaryPath: string;
    newVersion: string;
    ports: {
      cloud: number;
      site: number;
    };
    adminKey: string;
  },
): Promise<{ cleanupHandle: string }> {
  const { binaryPath: oldBinaryPath } = await ensureBackendBinaryDownloaded(
    ctx,
    {
      kind: "version",
      version: args.oldVersion,
    },
  );

  logVerbose(ctx, "Running backend on old version");
  const { cleanupHandle: oldCleanupHandle } = await runLocalBackend(ctx, {
    binaryPath: oldBinaryPath,
    ports: args.ports,
    deploymentName: args.deploymentName,
  });

  logVerbose(ctx, "Downloading env vars");
  const deploymentUrl = localDeploymentUrl(args.ports.cloud);
  const envs = (await runSystemQuery(ctx, {
    deploymentUrl,
    adminKey: args.adminKey,
    functionName: "_system/cli/queryEnvironmentVariables",
    componentPath: undefined,
    args: {},
  })) as Array<{
    name: string;
    value: string;
  }>;

  logVerbose(ctx, "Doing a snapshot export");
  const exportPath = path.join(
    deploymentStateDir(args.deploymentName),
    "export.zip",
  );
  if (ctx.fs.exists(exportPath)) {
    ctx.fs.unlink(exportPath);
  }
  const snaphsotExportState = await startSnapshotExport(ctx, {
    deploymentUrl,
    adminKey: args.adminKey,
    includeStorage: true,
    inputPath: exportPath,
  });
  if (snaphsotExportState.state !== "completed") {
    return ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: "Failed to export snapshot",
    });
  }
  await downloadSnapshotExport(ctx, {
    snapshotExportTs: snaphsotExportState.start_ts,
    inputPath: exportPath,
    adminKey: args.adminKey,
    deploymentUrl,
  });

  logVerbose(ctx, "Stopping the backend on the old version");
  const oldCleanupFunc = ctx.removeCleanup(oldCleanupHandle);
  if (oldCleanupFunc) {
    await oldCleanupFunc(0);
  }
  await ensureBackendStopped(ctx, {
    ports: args.ports,
    maxTimeSecs: 5,
    deploymentName: args.deploymentName,
    allowOtherDeployments: false,
  });

  // TODO(ENG-7078) save old artifacts to backup files
  logVerbose(ctx, "Running backend on new version");
  const { cleanupHandle } = await runLocalBackend(ctx, {
    binaryPath: args.newBinaryPath,
    ports: args.ports,
    deploymentName: args.deploymentName,
  });

  logVerbose(ctx, "Importing the env vars");
  if (envs.length > 0) {
    const fetch = deploymentFetch(ctx, {
      deploymentUrl,
      adminKey: args.adminKey,
    });
    try {
      await fetch("/api/update_environment_variables", {
        body: JSON.stringify({ changes: envs }),
        method: "POST",
      });
    } catch (e) {
      // TODO: this should ideally have a `LocalDeploymentError`
      return await logAndHandleFetchError(ctx, e);
    }
  }

  logVerbose(ctx, "Doing a snapshot import");
  const importId = await uploadForImport(ctx, {
    deploymentUrl,
    adminKey: args.adminKey,
    filePath: exportPath,
    importArgs: { format: "zip", mode: "replace", tableName: undefined },
    onImportFailed: async (e) => {
      logFailure(ctx, `Failed to import snapshot: ${e}`);
    },
  });
  logVerbose(ctx, `Snapshot import started`);
  let status = await waitForStableImportState(ctx, {
    importId,
    deploymentUrl,
    adminKey: args.adminKey,
    onProgress: () => {
      // do nothing for now
      return 0;
    },
  });
  if (status.state !== "waiting_for_confirmation") {
    const message = "Error while transferring data: Failed to upload snapshot";
    return ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: message,
      errForSentry: new LocalDeploymentError(message),
    });
  }

  await confirmImport(ctx, {
    importId,
    adminKey: args.adminKey,
    deploymentUrl,
    onError: async (e) => {
      logFailure(ctx, `Failed to confirm import: ${e}`);
    },
  });
  logVerbose(ctx, `Snapshot import confirmed`);
  status = await waitForStableImportState(ctx, {
    importId,
    deploymentUrl,
    adminKey: args.adminKey,
    onProgress: () => {
      // do nothing for now
      return 0;
    },
  });
  logVerbose(ctx, `Snapshot import status: ${status.state}`);
  if (status.state !== "completed") {
    const message = "Error while transferring data: Failed to import snapshot";
    return ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: message,
      errForSentry: new LocalDeploymentError(message),
    });
  }

  logFinishedStep(ctx, "Successfully upgraded to a new backend version");
  saveDeploymentConfig(ctx, args.deploymentName, {
    ports: args.ports,
    backendVersion: args.newVersion,
    adminKey: args.adminKey,
  });

  return { cleanupHandle };
}
