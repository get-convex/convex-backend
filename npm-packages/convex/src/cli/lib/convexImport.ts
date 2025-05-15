import chalk from "chalk";
import {
  formatSize,
  waitUntilCalled,
  deploymentFetch,
  logAndHandleFetchError,
} from "./utils/utils.js";
import {
  logFailure,
  Context,
  showSpinner,
  logFinishedStep,
  logWarning,
  logMessage,
  stopSpinner,
  changeSpinner,
} from "../../bundler/context.js";
import path from "path";
import { subscribe } from "./run.js";
import { ConvexHttpClient } from "../../browser/http_client.js";
import { makeFunctionReference } from "../../server/index.js";
import { promptYesNo } from "./utils/prompts.js";

// Backend has minimum chunk size of 5MiB except for the last chunk,
// so we use 5MiB as highWaterMark which makes fs.ReadStream[asyncIterator]
// output 5MiB chunks before the last one. This value can be overridden by
// setting `CONVEX_IMPORT_CHUNK_SIZE` (bytes) in the environment.
const DEFAULT_CHUNK_SIZE = 5 * 1024 * 1024;
const ENV_CHUNK_SIZE = process.env.CONVEX_IMPORT_CHUNK_SIZE
  ? parseInt(process.env.CONVEX_IMPORT_CHUNK_SIZE, 10)
  : undefined;

export async function importIntoDeployment(
  ctx: Context,
  filePath: string,
  options: {
    deploymentUrl: string;
    adminKey: string;
    deploymentNotice: string;
    snapshotImportDashboardLink: string | undefined;
    table?: string;
    format?: "csv" | "jsonLines" | "jsonArray" | "zip";
    replace?: boolean;
    append?: boolean;
    replaceAll?: boolean;
    yes?: boolean;
    component?: string;
  },
) {
  if (!ctx.fs.exists(filePath)) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "invalid filesystem data",
      printedMessage: `Error: Path ${chalk.bold(filePath)} does not exist.`,
    });
  }

  const format = await determineFormat(ctx, filePath, options.format ?? null);
  const tableName = options.table ?? null;
  if (tableName === null) {
    if (format !== "zip") {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: `Error: The \`--table\` option is required for format ${format}`,
      });
    }
  } else {
    if (format === "zip") {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: `Error: The \`--table\` option is not allowed for format ${format}`,
      });
    }
  }

  const convexClient = new ConvexHttpClient(options.deploymentUrl);
  convexClient.setAdminAuth(options.adminKey);
  const existingImports = await convexClient.query(
    makeFunctionReference<"query", Record<string, never>, Array<unknown>>(
      "_system/cli/queryImport:list",
    ),
    {},
  );
  const ongoingImports = existingImports.filter(
    (i) => (i as any).state.state === "in_progress",
  );
  if (ongoingImports.length > 0) {
    await askToConfirmImportWithExistingImports(
      ctx,
      options.snapshotImportDashboardLink,
      options.yes,
    );
  }

  const fileStats = ctx.fs.stat(filePath);
  showSpinner(ctx, `Importing ${filePath} (${formatSize(fileStats.size)})`);

  let mode = "requireEmpty";
  if (options.append) {
    mode = "append";
  } else if (options.replace) {
    mode = "replace";
  } else if (options.replaceAll) {
    mode = "replaceAll";
  }
  const importArgs = {
    tableName: tableName === null ? undefined : tableName,
    componentPath: options.component,
    mode,
    format,
  };
  const tableNotice = tableName ? ` to table "${chalk.bold(tableName)}"` : "";
  const onFailure = async () => {
    logFailure(
      ctx,
      `Importing data from "${chalk.bold(
        filePath,
      )}"${tableNotice}${options.deploymentNotice} failed`,
    );
  };
  const importId = await uploadForImport(ctx, {
    deploymentUrl: options.deploymentUrl,
    adminKey: options.adminKey,
    filePath,
    importArgs,
    onImportFailed: onFailure,
  });
  changeSpinner(ctx, "Parsing uploaded data");
  const onProgress = (
    ctx: Context,
    state: InProgressImportState,
    checkpointCount: number,
  ) => {
    stopSpinner(ctx);
    while ((state.checkpoint_messages?.length ?? 0) > checkpointCount) {
      logFinishedStep(ctx, state.checkpoint_messages![checkpointCount]);
      checkpointCount += 1;
    }
    showSpinner(ctx, state.progress_message ?? "Importing");
    return checkpointCount;
  };
  while (true) {
    const snapshotImportState = await waitForStableImportState(ctx, {
      importId,
      deploymentUrl: options.deploymentUrl,
      adminKey: options.adminKey,
      onProgress,
    });
    switch (snapshotImportState.state) {
      case "completed":
        logFinishedStep(
          ctx,
          `Added ${snapshotImportState.num_rows_written} documents${tableNotice}${options.deploymentNotice}.`,
        );
        return;
      case "failed":
        return await ctx.crash({
          exitCode: 1,
          errorType: "fatal",
          printedMessage: `Importing data from "${chalk.bold(
            filePath,
          )}"${tableNotice}${options.deploymentNotice} failed\n\n${chalk.red(snapshotImportState.error_message)}`,
        });
      case "waiting_for_confirmation": {
        // Clear spinner state so we can log and prompt without clobbering lines.
        stopSpinner(ctx);
        await askToConfirmImport(
          ctx,
          snapshotImportState.message_to_confirm,
          snapshotImportState.require_manual_confirmation,
          options.yes,
        );
        showSpinner(ctx, `Importing`);
        await confirmImport(ctx, {
          importId,
          adminKey: options.adminKey,
          deploymentUrl: options.deploymentUrl,
          onError: async () => {
            logFailure(
              ctx,
              `Importing data from "${chalk.bold(
                filePath,
              )}"${tableNotice}${options.deploymentNotice} failed`,
            );
          },
        });
        // Now we have kicked off the rest of the import, go around the loop again.
        break;
      }
      case "uploaded": {
        return await ctx.crash({
          exitCode: 1,
          errorType: "fatal",
          printedMessage: `Import canceled while parsing uploaded file`,
        });
      }
      case "in_progress": {
        const visitDashboardLink = options.snapshotImportDashboardLink
          ? ` Visit ${options.snapshotImportDashboardLink} to monitor its progress.`
          : "";
        return await ctx.crash({
          exitCode: 1,
          errorType: "fatal",
          printedMessage: `WARNING: Import is continuing to run on the server.${visitDashboardLink}`,
        });
      }
      default: {
        const _: never = snapshotImportState;
        return await ctx.crash({
          exitCode: 1,
          errorType: "fatal",
          printedMessage: `unknown error: unexpected state ${snapshotImportState as any}`,
          errForSentry: `unexpected snapshot import state ${(snapshotImportState as any).state}`,
        });
      }
    }
  }
}

async function askToConfirmImport(
  ctx: Context,
  messageToConfirm: string | undefined,
  requireManualConfirmation: boolean | undefined,
  yes: boolean | undefined,
) {
  if (!messageToConfirm?.length) {
    return;
  }
  logMessage(ctx, messageToConfirm);
  if (requireManualConfirmation !== false && !yes) {
    const confirmed = await promptYesNo(ctx, {
      message: "Perform import?",
      default: true,
    });
    if (!confirmed) {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: "Import canceled",
      });
    }
  }
}

async function askToConfirmImportWithExistingImports(
  ctx: Context,
  snapshotImportDashboardLink: string | undefined,
  yes: boolean | undefined,
) {
  const atDashboardLink = snapshotImportDashboardLink
    ? ` You can view its progress at ${snapshotImportDashboardLink}.`
    : "";
  logMessage(
    ctx,
    `There is already a snapshot import in progress.${atDashboardLink}`,
  );
  if (yes) {
    return;
  }
  const confirmed = await promptYesNo(ctx, {
    message: "Start another import?",
    default: true,
  });
  if (!confirmed) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: "Import canceled",
    });
  }
}

type InProgressImportState = {
  state: "in_progress";
  progress_message?: string | undefined;
  checkpoint_messages?: string[] | undefined;
};

type SnapshotImportState =
  | { state: "uploaded" }
  | {
      state: "waiting_for_confirmation";
      message_to_confirm?: string;
      require_manual_confirmation?: boolean;
    }
  | InProgressImportState
  | { state: "completed"; num_rows_written: bigint }
  | { state: "failed"; error_message: string };

export async function waitForStableImportState(
  ctx: Context,
  args: {
    importId: string;
    deploymentUrl: string;
    adminKey: string;
    onProgress: (
      ctx: Context,
      state: InProgressImportState,
      checkpointCount: number,
    ) => number;
  },
): Promise<SnapshotImportState> {
  const { importId, deploymentUrl, adminKey, onProgress } = args;
  const [donePromise, onDone] = waitUntilCalled();
  let snapshotImportState: SnapshotImportState;
  let checkpointCount = 0;
  await subscribe(ctx, {
    deploymentUrl,
    adminKey,
    parsedFunctionName: "_system/cli/queryImport",
    parsedFunctionArgs: { importId },
    componentPath: undefined,
    until: donePromise,
    callbacks: {
      onChange: (value: any) => {
        snapshotImportState = value.state;
        switch (snapshotImportState.state) {
          case "waiting_for_confirmation":
          case "completed":
          case "failed":
            onDone();
            break;
          case "uploaded":
            // Not a stable state. Ignore while the server continues working.
            return;
          case "in_progress":
            // Not a stable state. Ignore while the server continues working.
            checkpointCount = onProgress(
              ctx,
              snapshotImportState,
              checkpointCount,
            );
            return;
        }
      },
    },
  });
  return snapshotImportState!;
}

async function determineFormat(
  ctx: Context,
  filePath: string,
  format: string | null,
) {
  const fileExtension = path.extname(filePath);
  if (fileExtension !== "") {
    const formatToExtension: Record<string, string> = {
      csv: ".csv",
      jsonLines: ".jsonl",
      jsonArray: ".json",
      zip: ".zip",
    };
    const extensionToFormat = Object.fromEntries(
      Object.entries(formatToExtension).map((a) => a.reverse()),
    );
    if (format !== null && fileExtension !== formatToExtension[format]) {
      logWarning(
        ctx,
        chalk.yellow(
          `Warning: Extension of file ${filePath} (${fileExtension}) does not match specified format: ${format} (${formatToExtension[format]}).`,
        ),
      );
    }
    format ??= extensionToFormat[fileExtension] ?? null;
  }
  if (format === null) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage:
        "No input file format inferred by the filename extension or specified. Specify your input file's format using the `--format` flag.",
    });
  }
  return format;
}

export async function confirmImport(
  ctx: Context,
  args: {
    importId: string;
    adminKey: string;
    deploymentUrl: string;
    onError: (e: any) => Promise<void>;
  },
) {
  const { importId, adminKey, deploymentUrl } = args;
  const fetch = deploymentFetch(ctx, {
    deploymentUrl,
    adminKey,
  });
  const performUrl = `/api/perform_import`;
  try {
    await fetch(performUrl, {
      method: "POST",
      body: JSON.stringify({ importId }),
    });
  } catch (e) {
    await args.onError(e);
    return await logAndHandleFetchError(ctx, e);
  }
}

export async function uploadForImport(
  ctx: Context,
  args: {
    deploymentUrl: string;
    adminKey: string;
    filePath: string;
    importArgs: {
      tableName?: string;
      componentPath?: string;
      mode: string;
      format: string;
    };
    onImportFailed: (e: any) => Promise<void>;
  },
) {
  const { deploymentUrl, adminKey, filePath } = args;
  const fetch = deploymentFetch(ctx, {
    deploymentUrl,
    adminKey,
  });

  const fileStats = ctx.fs.stat(filePath);
  // The backend rejects uploads of 10k or more parts. We use 9999 instead of
  // 10000 so rounding errors can't push us over the limit.
  const minChunkSize = Math.ceil(fileStats.size / 9999);
  let chunkSize = ENV_CHUNK_SIZE ?? DEFAULT_CHUNK_SIZE;
  if (chunkSize < minChunkSize) {
    chunkSize = minChunkSize;
  }
  const data = ctx.fs.createReadStream(filePath, {
    highWaterMark: chunkSize,
  });

  showSpinner(ctx, `Importing ${filePath} (${formatSize(fileStats.size)})`);
  let importId: string;
  try {
    const startResp = await fetch("/api/import/start_upload", {
      method: "POST",
    });
    const { uploadToken } = await startResp.json();

    const partTokens = [];
    let partNumber = 1;

    for await (const chunk of data) {
      const partUrl = `/api/import/upload_part?uploadToken=${encodeURIComponent(
        uploadToken,
      )}&partNumber=${partNumber}`;
      const partResp = await fetch(partUrl, {
        headers: {
          "Content-Type": "application/octet-stream",
        },
        body: chunk,
        method: "POST",
      });
      partTokens.push(await partResp.json());
      partNumber += 1;
      changeSpinner(
        ctx,
        `Uploading ${filePath} (${formatSize(data.bytesRead)}/${formatSize(
          fileStats.size,
        )})`,
      );
    }

    const finishResp = await fetch("/api/import/finish_upload", {
      body: JSON.stringify({
        import: args.importArgs,
        uploadToken,
        partTokens,
      }),
      method: "POST",
    });
    const body = await finishResp.json();
    importId = body.importId;
  } catch (e) {
    await args.onImportFailed(e);
    return await logAndHandleFetchError(ctx, e);
  }
  return importId;
}
