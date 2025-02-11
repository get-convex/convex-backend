import { Command } from "@commander-js/extra-typings";
import chalk from "chalk";
import { ensureHasConvexDependency } from "./lib/utils/utils.js";
import { oneoffContext } from "../bundler/context.js";
import {
  fetchDeploymentCredentialsProvisionProd,
  deploymentSelectionFromOptions,
} from "./lib/api.js";
import { deploymentDashboardUrlPage } from "./dashboard.js";
import { actionDescription } from "./lib/command.js";
import { exportFromDeployment } from "./lib/convexExport.js";

export const convexExport = new Command("export")
  .summary("Export data from your deployment to a ZIP file")
  .description(
    "Export data, and optionally file storage, from your Convex deployment to a ZIP file.\n" +
      "By default, this exports from your dev deployment.",
  )
  .allowExcessArguments(false)
  .addExportOptions()
  .addDeploymentSelectionOptions(actionDescription("Export data from"))
  .showHelpAfterError()
  .action(async (options) => {
    const ctx = oneoffContext();

    const deploymentSelection = await deploymentSelectionFromOptions(
      ctx,
      options,
    );

    const {
      adminKey,
      url: deploymentUrl,
      deploymentName,
    } = await fetchDeploymentCredentialsProvisionProd(ctx, deploymentSelection);

    await ensureHasConvexDependency(ctx, "export");

    const deploymentNotice = options.prod
      ? ` in your ${chalk.bold("prod")} deployment`
      : "";
    await exportFromDeployment(ctx, {
      ...options,
      deploymentUrl,
      adminKey,
      deploymentNotice,
      snapshotExportDashboardLink: deploymentDashboardUrlPage(
        deploymentName ?? null,
        "/settings/snapshot-export",
      ),
    });
  });
