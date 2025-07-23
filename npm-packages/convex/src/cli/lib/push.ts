import chalk from "chalk";
import {
  Context,
  changeSpinner,
  logFinishedStep,
  logMessage,
} from "../../bundler/context.js";
import { doCodegen } from "./codegen.js";
import {
  ProjectConfig,
  configFromProjectConfig,
  diffConfig,
  debugIsolateEndpointBundles,
  pullConfig,
  pushConfig,
} from "./config.js";
import { pushSchema } from "./indexes.js";
import { typeCheckFunctionsInMode } from "./typecheck.js";
import { ensureHasConvexDependency, functionsDir } from "./utils/utils.js";
import { handleDebugBundlePath } from "./debugBundlePath.js";

import { LogManager } from "./logs.js";

export type PushOptions = {
  adminKey: string;
  verbose: boolean;
  dryRun: boolean;
  typecheck: "enable" | "try" | "disable";
  typecheckComponents: boolean;
  debug: boolean;
  debugBundlePath?: string;
  debugNodeApis: boolean;
  codegen: boolean;
  url: string;
  deploymentName: string | null;
  writePushRequest?: string;
  liveComponentSources: boolean;
  logManager?: LogManager;
};

export async function runNonComponentsPush(
  ctx: Context,
  options: PushOptions,
  configPath: string,
  projectConfig: ProjectConfig,
) {
  if (options.writePushRequest) {
    logMessage(
      ctx,
      "Skipping push because --write-push-request is set, but we are on the non-components path so there is nothing to write.",
    );
    return;
  }
  const timeRunPushStarts = performance.now();
  const origin = options.url;
  const verbose = options.verbose || options.dryRun;
  if (verbose) {
    process.env["CONVEX_VERBOSE"] = "1";
  }
  await ensureHasConvexDependency(ctx, "push");

  if (!options.codegen) {
    logMessage(
      ctx,
      chalk.gray("Skipping codegen. Remove --codegen=disable to enable."),
    );
    // Codegen includes typechecking, so if we're skipping it, run the type
    // check manually on the query and mutation functions
    const funcDir = functionsDir(configPath, projectConfig);
    await typeCheckFunctionsInMode(ctx, options.typecheck, funcDir);
  } else {
    await doCodegen(
      ctx,
      functionsDir(configPath, projectConfig),
      options.typecheck,
      options,
    );
    if (verbose) {
      logMessage(ctx, chalk.green("Codegen finished."));
    }
  }

  if (options.debugNodeApis) {
    await debugIsolateEndpointBundles(ctx, projectConfig, configPath);
    logFinishedStep(
      ctx,
      "All non-'use node' entry points successfully bundled. Skipping rest of push.",
    );
    return;
  }
  const timeBundleStarts = performance.now();

  const { config: localConfig, bundledModuleInfos } =
    await configFromProjectConfig(ctx, projectConfig, configPath, verbose);

  if (options.debugBundlePath) {
    await handleDebugBundlePath(ctx, options.debugBundlePath, localConfig);
    logMessage(
      ctx,
      `Wrote bundle and metadata to ${options.debugBundlePath}. Skipping rest of push.`,
    );
    return;
  }

  const timeSchemaPushStarts = performance.now();
  const { schemaId, schemaState } = await pushSchema(
    ctx,
    origin,
    options.adminKey,
    functionsDir(configPath, localConfig.projectConfig),
    options.dryRun,
    options.deploymentName,
  );

  const timeConfigPullStarts = performance.now();
  const remoteConfigWithModuleHashes = await pullConfig(
    ctx,
    undefined,
    undefined,
    origin,
    options.adminKey,
  );

  changeSpinner(ctx, "Diffing local code and deployment state");
  const { diffString, stats } = diffConfig(
    remoteConfigWithModuleHashes,
    localConfig,
  );
  if (diffString === "" && schemaState?.state === "active") {
    if (verbose) {
      const msg =
        localConfig.modules.length === 0
          ? `No functions found in ${localConfig.projectConfig.functions}`
          : "Config already synced";
      logMessage(
        ctx,
        chalk.gray(
          `${
            options.dryRun
              ? "Command would skip function push"
              : "Function push skipped"
          }: ${msg}.`,
        ),
      );
    }
    return;
  }

  if (verbose) {
    logMessage(
      ctx,
      chalk.bold(
        `Remote config ${
          options.dryRun ? "would" : "will"
        } be overwritten with the following changes:`,
      ),
    );
    logMessage(ctx, diffString);
  }

  if (options.dryRun) {
    return;
  }

  // Note that this is not quite a user pain metric: we're missing any time
  // spent making and retrying this network request and receiving the response.
  const timePushStarts = performance.now();
  const timing = {
    typecheck: (timeBundleStarts - timeRunPushStarts) / 1000,
    bundle: (timeSchemaPushStarts - timeBundleStarts) / 1000,
    schemaPush: (timeConfigPullStarts - timeSchemaPushStarts) / 1000,
    codePull: (timePushStarts - timeConfigPullStarts) / 1000,
    totalBeforePush: (timePushStarts - timeRunPushStarts) / 1000,
    moduleDiffStats: stats,
  };
  await pushConfig(ctx, localConfig, {
    adminKey: options.adminKey,
    url: options.url,
    deploymentName: options.deploymentName,
    pushMetrics: timing,
    schemaId,
    bundledModuleInfos,
  });
}
