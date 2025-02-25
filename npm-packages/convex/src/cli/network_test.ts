import { Command } from "@commander-js/extra-typings";
import {
  DeploymentSelection,
  deploymentSelectionFromOptions,
  fetchDeploymentCredentialsProvisionProd,
} from "./lib/api.js";
import {
  Context,
  logMessage,
  oneoffContext,
  showSpinner,
} from "../bundler/context.js";
import chalk from "chalk";
import { actionDescription } from "./lib/command.js";
import { runNetworkTestOnUrl, withTimeout } from "./lib/networkTest.js";

export const networkTest = new Command("network-test")
  .description("Run a network test to Convex's servers")
  .allowExcessArguments(false)
  .addNetworkTestOptions()
  .addDeploymentSelectionOptions(
    actionDescription("Perform the network test on"),
  )
  .option("--url <url>") // unhide help
  .action(async (options) => {
    const ctx = oneoffContext();
    const timeoutSeconds = options.timeout
      ? Number.parseFloat(options.timeout)
      : 30;
    await withTimeout(
      ctx,
      "Network test",
      timeoutSeconds * 1000,
      runNetworkTest(ctx, options),
    );
  });

async function runNetworkTest(
  ctx: Context,
  options: {
    prod?: boolean | undefined;
    previewName?: string | undefined;
    deploymentName?: string | undefined;
    url?: string | undefined;
    adminKey?: string | undefined;
    ipFamily?: string;
    speedTest?: boolean;
  },
) {
  showSpinner(ctx, "Performing network test...");
  const deploymentSelection = await deploymentSelectionFromOptions(
    ctx,
    options,
  );
  const { url, adminKey } = await loadUrlAndAdminKey(ctx, deploymentSelection);
  await runNetworkTestOnUrl(ctx, { url, adminKey }, options);
}

async function loadUrlAndAdminKey(
  ctx: Context,
  deploymentSelection: DeploymentSelection,
): Promise<{ url: string; adminKey: string | null }> {
  // Try to fetch the URL following the usual paths, but special case the
  // `--url` argument in case the developer doesn't have network connectivity.
  let url: string;
  let adminKey: string | null;
  if (deploymentSelection.kind === "urlWithAdminKey") {
    url = deploymentSelection.url;
    adminKey = deploymentSelection.adminKey;
  } else if (deploymentSelection.kind === "urlWithLogin") {
    url = deploymentSelection.url;
    adminKey = null;
  } else {
    const credentials = await fetchDeploymentCredentialsProvisionProd(
      ctx,
      deploymentSelection,
    );
    url = credentials.url;
    adminKey = credentials.adminKey;
  }
  logMessage(ctx, `${chalk.green(`✔`)} Deployment URL: ${url}`);
  return { url, adminKey };
}
