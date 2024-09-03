import chalk from "chalk";
import {
  ensureHasConvexDependency,
  formatSize,
  waitUntilCalled,
  deploymentFetch,
  logAndHandleFetchError,
} from "./lib/utils/utils.js";
import {
  logFailure,
  oneoffContext,
  Context,
  showSpinner,
  logFinishedStep,
  logWarning,
  logMessage,
  stopSpinner,
  changeSpinner,
} from "../bundler/context.js";
import {
  fetchDeploymentCredentialsProvisionProd,
  deploymentSelectionFromOptions,
} from "./lib/api.js";
import path from "path";
import { subscribe } from "./lib/run.js";
import { Command, Option } from "@commander-js/extra-typings";
import { actionDescription } from "./lib/command.js";
import { ConvexHttpClient } from "../browser/http_client.js";
import { makeFunctionReference } from "../server/index.js";
import { deploymentDashboardUrlPage } from "./dashboard.js";
import { promptYesNo } from "./lib/utils/prompts.js";

// Backend has minimum chunk size of 5MiB except for the last chunk,
// so we use 5MiB as highWaterMark which makes fs.ReadStream[asyncIterator]
// output 5MiB chunks before the last one.
const CHUNK_SIZE = 5 * 1024 * 1024;

export const convexImport = new Command("import")
  .summary("Import data from a file to your deployment")
  .description(
    "Import data from a file to your Convex deployment.\n\n" +
      "  From a snapshot: `npx convex import snapshot.zip`\n" +
      "  For a single table: `npx convex import --table tableName file.json`\n\n" +
      "By default, this imports into your dev deployment.",
  )
  .addOption(
    new Option(
      "--table <table>",
      "Destination table name. Required if format is csv, jsonLines, or jsonArray. Not supported if format is zip.",
    ),
  )
  .addOption(
    new Option(
      "--replace",
      "Replace all existing data in any of the imported tables",
    ).conflicts("--append"),
  )
  .addOption(
    new Option(
      "--append",
      "Append imported data to any existing tables",
    ).conflicts("--replace"),
  )
  .option(
    "-y, --yes",
    "Skip confirmation prompt when import leads to deleting existing documents",
  )
  .addOption(
    new Option(
      "--format <format>",
      "Input file format. This flag is only required if the filename is missing an extension.\n" +
        "- CSV files must have a header, and each row's entries are interpreted either as a (floating point) number or a string.\n" +
        "- JSON files must be an array of JSON objects.\n" +
        "- JSONLines files must have a JSON object per line.\n" +
        "- ZIP files must have one directory per table, containing <table>/documents.jsonl. Snapshot exports from the Convex dashboard have this format.",
    ).choices(["csv", "jsonLines", "jsonArray", "zip"]),
  )
  .addDeploymentSelectionOptions(actionDescription("Import data into"))
  .argument("<path>", "Path to the input file")
  .showHelpAfterError()
  .action(async (filePath, options, command) => {
    const ctx = oneoffContext;

    if (command.args.length > 1) {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: `Error: Too many positional arguments. If you're specifying a table name, use the \`--table\` option.`,
      });
    }

    const deploymentSelection = deploymentSelectionFromOptions(options);

    const {
      adminKey,
      url: deploymentUrl,
      deploymentName,
    } = await fetchDeploymentCredentialsProvisionProd(ctx, deploymentSelection);

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

    await ensureHasConvexDependency(ctx, "import");
    const convexClient = new ConvexHttpClient(deploymentUrl);
    convexClient.setAdminAuth(adminKey);
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
        deploymentName,
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
    }
    const importArgs = {
      tableName: tableName === null ? undefined : tableName,
      mode,
      format,
    };
    const deploymentNotice = options.prod
      ? ` in your ${chalk.bold("prod")} deployment`
      : "";
    const tableNotice = tableName ? ` to table "${chalk.bold(tableName)}"` : "";
    const onFailure = async () => {
      logFailure(
        ctx,
        `Importing data from "${chalk.bold(
          filePath,
        )}"${tableNotice}${deploymentNotice} failed`,
      );
    };
    const importId = await uploadForImport(ctx, {
      deploymentUrl,
      adminKey,
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
        deploymentUrl,
        adminKey,
        onProgress,
      });
      switch (snapshotImportState.state) {
        case "completed":
          logFinishedStep(
            ctx,
            `Added ${snapshotImportState.num_rows_written} documents${tableNotice}${deploymentNotice}.`,
          );
          return;
        case "failed":
          return await ctx.crash({
            exitCode: 1,
            errorType: "fatal",
            printedMessage: `Importing data from "${chalk.bold(
              filePath,
            )}"${tableNotice}${deploymentNotice} failed\n\n${chalk.red(snapshotImportState.error_message)}`,
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
            adminKey,
            deploymentUrl,
            onError: async () => {
              logFailure(
                ctx,
                `Importing data from "${chalk.bold(
                  filePath,
                )}"${tableNotice}${deploymentNotice} failed`,
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
          return await ctx.crash({
            exitCode: 1,
            errorType: "fatal",
            printedMessage: `WARNING: Import is continuing to run on the server. Visit ${snapshotImportDashboardLink(deploymentName)} to monitor its progress.`,
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
  });

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

function snapshotImportDashboardLink(deploymentName: string | undefined) {
  return deploymentName === undefined
    ? "https://dashboard.convex.dev/d/settings/snapshot-export"
    : deploymentDashboardUrlPage(deploymentName, "/settings/snapshot-export");
}

async function askToConfirmImportWithExistingImports(
  ctx: Context,
  deploymentName: string | undefined,
  yes: boolean | undefined,
) {
  logMessage(
    ctx,
    `There is already a snapshot import in progress. You can view its progress at ${snapshotImportDashboardLink(deploymentName)}.`,
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
  await subscribe(
    ctx,
    deploymentUrl,
    adminKey,
    "_system/cli/queryImport",
    { importId },
    donePromise,
    {
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
  );
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
  const fetch = deploymentFetch(deploymentUrl, adminKey);
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
    importArgs: { tableName?: string; mode: string; format: string };
    onImportFailed: (e: any) => Promise<void>;
  },
) {
  const { deploymentUrl, adminKey, filePath } = args;
  const fetch = deploymentFetch(deploymentUrl, adminKey);

  const data = ctx.fs.createReadStream(filePath, {
    highWaterMark: CHUNK_SIZE,
  });
  const fileStats = ctx.fs.stat(filePath);

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
