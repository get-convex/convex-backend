import { Command, Option } from "@commander-js/extra-typings";
import chalk from "chalk";
import {
  ensureHasConvexDependency,
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
  logError,
  stopSpinner,
  changeSpinner,
} from "../bundler/context.js";
import {
  fetchDeploymentCredentialsProvisionProd,
  deploymentSelectionFromOptions,
} from "./lib/api.js";
import { subscribe } from "./lib/run.js";
import { nodeFs } from "../bundler/fs.js";
import path from "path";
import { deploymentDashboardUrlPage } from "./dashboard.js";
import { actionDescription } from "./lib/command.js";
import { Readable } from "stream";

export const convexExport = new Command("export")
  .summary("Export data from your deployment to a ZIP file")
  .description(
    "Export data, and optionally file storage, from your Convex deployment to a ZIP file.\n" +
      "By default, this exports from your dev deployment.",
  )
  .requiredOption(
    "--path <zipFilePath>",
    "Exports data into a ZIP file at this path, which may be a directory or unoccupied .zip path",
  )
  .addOption(
    new Option(
      "--include-file-storage",
      "Includes stored files (https://dashboard.convex.dev/deployment/files) in a _storage folder within the ZIP file",
    ),
  )
  .addDeploymentSelectionOptions(actionDescription("Export data from"))
  .showHelpAfterError()
  .action(async (options) => {
    const ctx = oneoffContext;

    const deploymentSelection = deploymentSelectionFromOptions(options);

    const {
      adminKey,
      url: deploymentUrl,
      deploymentName,
    } = await fetchDeploymentCredentialsProvisionProd(ctx, deploymentSelection);

    const inputPath = options.path;
    const includeStorage = !!options.includeFileStorage;

    await ensureHasConvexDependency(ctx, "export");

    const deploymentNotice = options.prod
      ? ` in your ${chalk.bold("prod")} deployment`
      : "";
    showSpinner(ctx, `Creating snapshot export${deploymentNotice}`);

    const fetch = deploymentFetch(deploymentUrl);
    const headers = {
      Authorization: `Convex ${adminKey}`,
      "Convex-Client": `npm-cli-${version}`,
    };
    try {
      await fetch(`/api/export/request/zip?includeStorage=${includeStorage}`, {
        method: "POST",
        headers,
      });
    } catch (e) {
      return await logAndHandleFetchError(ctx, e);
    }

    const snapshotExportState = await waitForStableExportState(
      ctx,
      deploymentUrl,
      adminKey,
    );
    switch (snapshotExportState.state) {
      case "completed":
        stopSpinner(ctx);
        logFinishedStep(
          ctx,
          `Created snapshot export at timestamp ${snapshotExportState.start_ts}`,
        );
        logFinishedStep(
          ctx,
          `Export is available at ${await deploymentDashboardUrlPage(
            deploymentName ?? null,
            "/settings/snapshot-export",
          )}`,
        );
        break;
      case "requested":
      case "in_progress": {
        logFailure(ctx, `WARNING: Export is continuing to run on the server.`);
        return await ctx.crash(1);
      }
      default: {
        const _: never = snapshotExportState;
        logFailure(
          ctx,
          `unknown error: unexpected state ${snapshotExportState as any}`,
        );
        return await ctx.crash(1);
      }
    }

    showSpinner(ctx, `Downloading snapshot export to ${chalk.bold(inputPath)}`);
    const exportUrl = `/api/export/zip/${snapshotExportState.start_ts.toString()}?adminKey=${encodeURIComponent(
      adminKey,
    )}`;
    let response: Response;
    try {
      response = await fetch(exportUrl, {
        method: "GET",
        headers,
      });
    } catch (e) {
      return await logAndHandleFetchError(ctx, e);
    }

    let filePath;
    if (ctx.fs.exists(inputPath)) {
      const st = ctx.fs.stat(inputPath);
      if (st.isDirectory()) {
        const contentDisposition =
          response.headers.get("content-disposition") ?? "";
        let filename = `snapshot_${snapshotExportState.start_ts.toString()}.zip`;
        if (contentDisposition.startsWith("attachment; filename=")) {
          filename = contentDisposition.slice("attachment; filename=".length);
        }
        filePath = path.join(inputPath, filename);
      } else {
        logFailure(ctx, `Error: Path ${chalk.bold(inputPath)} already exists.`);
        return await ctx.crash(1, "invalid filesystem data");
      }
    } else {
      filePath = inputPath;
    }
    changeSpinner(
      ctx,
      `Downloading snapshot export to ${chalk.bold(filePath)}`,
    );

    try {
      await nodeFs.writeFileStream(
        filePath,
        Readable.fromWeb(response.body! as any),
      );
    } catch (e) {
      logFailure(ctx, `Exporting data failed`);
      logError(ctx, chalk.red(e));
      return await ctx.crash(1);
    }
    stopSpinner(ctx);
    logFinishedStep(
      ctx,
      `Downloaded snapshot export to ${chalk.bold(filePath)}`,
    );
  });

type SnapshotExportState =
  | { state: "requested" }
  | { state: "in_progress" }
  | {
      state: "completed";
      complete_ts: bigint;
      start_ts: bigint;
      zip_object_key: string;
    };

async function waitForStableExportState(
  ctx: Context,
  deploymentUrl: string,
  adminKey: string,
): Promise<SnapshotExportState> {
  const [donePromise, onDone] = waitUntilCalled();
  let snapshotExportState: SnapshotExportState;
  await subscribe(
    ctx,
    deploymentUrl,
    adminKey,
    "_system/cli/exports:getLatest",
    {},
    donePromise,
    {
      onChange: (value: any) => {
        // NOTE: `value` would only be `null` if there has never been an export
        // requested.
        snapshotExportState = value;
        switch (snapshotExportState.state) {
          case "requested":
          case "in_progress":
            // Not a stable state.
            break;
          case "completed":
            onDone();
            break;
          default: {
            const _: never = snapshotExportState;
            onDone();
          }
        }
      },
    },
  );
  return snapshotExportState!;
}
