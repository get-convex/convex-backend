import chalk from "chalk";
import inquirer from "inquirer";
import {
  ensureHasConvexDependency,
  formatSize,
  waitUntilCalled,
  deploymentFetch,
  logAndHandleFetchError,
} from "./lib/utils.js";
import { version } from "./version.js";
import {
  logFailure,
  oneoffContext,
  Context,
  showSpinner,
  logFinishedStep,
  logWarning,
  logError,
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
      logFailure(
        ctx,
        `Error: Too many positional arguments. If you're specifying a table name, use the \`--table\` option.`,
      );
      return await ctx.crash(1, "fatal");
    }

    const deploymentSelection = deploymentSelectionFromOptions(options);

    const {
      adminKey,
      url: deploymentUrl,
      deploymentName,
    } = await fetchDeploymentCredentialsProvisionProd(ctx, deploymentSelection);

    if (!ctx.fs.exists(filePath)) {
      logFailure(ctx, `Error: Path ${chalk.bold(filePath)} does not exist.`);
      return await ctx.crash(1, "invalid filesystem data");
    }

    const format = await determineFormat(ctx, filePath, options.format ?? null);
    const tableName = options.table ?? null;
    if (tableName === null) {
      if (format !== "zip") {
        logFailure(
          ctx,
          `Error: The \`--table\` option is required for format ${format}`,
        );
        return await ctx.crash(1, "fatal");
      }
    } else {
      if (format === "zip") {
        logFailure(
          ctx,
          `Error: The \`--table\` option is not allowed for format ${format}`,
        );
        return await ctx.crash(1, "fatal");
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
    const fetch = deploymentFetch(deploymentUrl);

    const data = ctx.fs.createReadStream(filePath, {
      highWaterMark: CHUNK_SIZE,
    });
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
    const headers = {
      Authorization: `Convex ${adminKey}`,
      "Convex-Client": `npm-cli-${version}`,
    };
    const deploymentNotice = options.prod
      ? ` in your ${chalk.bold("prod")} deployment`
      : "";
    const tableNotice = tableName ? ` to table "${chalk.bold(tableName)}"` : "";
    let importId: string;
    try {
      const startResp = await fetch("/api/import/start_upload", {
        method: "POST",
        headers,
      });
      const { uploadToken } = await startResp.json();

      const partTokens = [];
      let partNumber = 1;

      for await (const chunk of data) {
        const partUrl = `/api/import/upload_part?uploadToken=${encodeURIComponent(
          uploadToken,
        )}&partNumber=${partNumber}`;
        const partResp = await fetch(partUrl, {
          headers,
          body: chunk,
          method: "POST",
        });
        partTokens.push(await partResp.text());
        partNumber += 1;
        changeSpinner(
          ctx,
          `Uploading ${filePath} (${formatSize(data.bytesRead)}/${formatSize(
            fileStats.size,
          )})`,
        );
      }

      const finishResp = await fetch("/api/import/finish_upload", {
        headers: {
          ...headers,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          import: importArgs,
          uploadToken,
          partTokens,
        }),
        method: "POST",
      });
      const body = await finishResp.json();
      importId = body.importId;
    } catch (e) {
      logFailure(
        ctx,
        `Importing data from "${chalk.bold(
          filePath,
        )}"${tableNotice}${deploymentNotice} failed`,
      );
      return await logAndHandleFetchError(ctx, e);
    }
    changeSpinner(ctx, "Parsing uploaded data");
    // eslint-disable-next-line no-constant-condition
    while (true) {
      const snapshotImportState = await waitForStableImportState(
        ctx,
        importId,
        deploymentUrl,
        adminKey,
      );
      switch (snapshotImportState.state) {
        case "completed":
          logFinishedStep(
            ctx,
            `Added ${snapshotImportState.num_rows_written} documents${tableNotice}${deploymentNotice}.`,
          );
          return;
        case "failed":
          logFailure(
            ctx,
            `Importing data from "${chalk.bold(
              filePath,
            )}"${tableNotice}${deploymentNotice} failed`,
          );
          logError(ctx, chalk.red(snapshotImportState.error_message));
          return await ctx.crash(1);
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
          const performUrl = `/api/perform_import`;
          try {
            await fetch(performUrl, {
              headers: { ...headers, "content-type": "application/json" },
              method: "POST",
              body: JSON.stringify({ importId }),
            });
          } catch (e) {
            logFailure(
              ctx,
              `Importing data from "${chalk.bold(
                filePath,
              )}"${tableNotice}${deploymentNotice} failed`,
            );
            return await logAndHandleFetchError(ctx, e);
          }
          // Now we have kicked off the rest of the import, go around the loop again.
          break;
        }
        case "uploaded": {
          logFailure(ctx, `Import canceled while parsing uploaded file`);
          return await ctx.crash(1);
        }
        case "in_progress": {
          logFailure(
            ctx,
            `WARNING: Import is continuing to run on the server. Visit ${snapshotImportDashboardLink(deploymentName)} to monitor its progress.`,
          );
          return await ctx.crash(1);
        }
        default: {
          const _: never = snapshotImportState;
          logFailure(
            ctx,
            `unknown error: unexpected state ${snapshotImportState as any}`,
          );
          return await ctx.crash(1);
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
    const { confirmed } = await inquirer.prompt([
      {
        type: "confirm",
        name: "confirmed",
        message: `Perform the import?`,
        default: true,
      },
    ]);
    if (!confirmed) {
      return await ctx.crash(1);
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
  const { confirmed } = await inquirer.prompt([
    {
      type: "confirm",
      name: "confirmed",
      message: `Start another import?`,
      default: true,
    },
  ]);
  if (!confirmed) {
    return await ctx.crash(1);
  }
}

type SnapshotImportState =
  | { state: "uploaded" }
  | {
      state: "waiting_for_confirmation";
      message_to_confirm?: string;
      require_manual_confirmation?: boolean;
    }
  | {
      state: "in_progress";
      progress_message?: string | undefined;
      checkpoint_messages?: string[] | undefined;
    }
  | { state: "completed"; num_rows_written: bigint }
  | { state: "failed"; error_message: string };

async function waitForStableImportState(
  ctx: Context,
  importId: string,
  deploymentUrl: string,
  adminKey: string,
): Promise<SnapshotImportState> {
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
            stopSpinner(ctx);
            while (
              (snapshotImportState.checkpoint_messages?.length ?? 0) >
              checkpointCount
            ) {
              logFinishedStep(
                ctx,
                snapshotImportState.checkpoint_messages![checkpointCount],
              );
              checkpointCount += 1;
            }
            showSpinner(
              ctx,
              snapshotImportState.progress_message ?? "Importing",
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
    logFailure(
      ctx,
      "No input file format inferred by the filename extension or specified. Specify your input file's format using the `--format` flag.",
    );
    return await ctx.crash(1, "fatal");
  }
  return format;
}
