import chalk from "chalk";
import { ensureHasConvexDependency } from "./lib/utils/utils.js";
import { oneoffContext } from "../bundler/context.js";
import {
  fetchDeploymentCredentialsProvisionProd,
  deploymentSelectionFromOptions,
} from "./lib/api.js";
import { Command } from "@commander-js/extra-typings";
import { actionDescription } from "./lib/command.js";
import { deploymentDashboardUrlPage } from "./dashboard.js";
import { importIntoDeployment } from "./lib/convexImport.js";

export const convexImport = new Command("import")
  .summary("Import data from a file to your deployment")
  .description(
    "Import data from a file to your Convex deployment.\n\n" +
      "  From a snapshot: `npx convex import snapshot.zip`\n" +
      "  For a single table: `npx convex import --table tableName file.json`\n\n" +
      "By default, this imports into your dev deployment.",
  )
  .allowExcessArguments(false)
  .addImportOptions()
  .addDeploymentSelectionOptions(actionDescription("Import data into"))
  .showHelpAfterError()
  .action(async (filePath, options) => {
    const ctx = oneoffContext();

    await ensureHasConvexDependency(ctx, "import");

    const deploymentSelection = deploymentSelectionFromOptions(options);

    const deploymentNotice = options.prod
      ? ` in your ${chalk.bold("prod")} deployment`
      : "";

    const {
      adminKey,
      url: deploymentUrl,
      deploymentName,
    } = await fetchDeploymentCredentialsProvisionProd(ctx, deploymentSelection);

    await importIntoDeployment(ctx, filePath, {
      ...options,
      deploymentUrl,
      adminKey,
      deploymentNotice,
      snapshotImportDashboardLink: snapshotImportDashboardLink(deploymentName),
    });
  });

function snapshotImportDashboardLink(deploymentName: string | undefined) {
  return deploymentName === undefined
    ? "https://dashboard.convex.dev/deployment/settings/snapshots"
    : deploymentDashboardUrlPage(deploymentName, "/settings/snapshots");
}
