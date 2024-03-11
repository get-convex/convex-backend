import chalk from "chalk";
import { Context, changeSpinner, logMessage } from "../../bundler/context.js";
import { doCodegen } from "./codegen.js";
import {
  configFromProjectConfig,
  configJSON,
  diffConfig,
  pullConfig,
  pushConfig,
  readProjectConfig,
} from "./config.js";
import { pushSchema } from "./indexes.js";
import { typeCheckFunctionsInMode } from "./typecheck.js";
import { ensureHasConvexDependency, functionsDir } from "./utils.js";

export type PushOptions = {
  adminKey: string;
  verbose: boolean;
  dryRun: boolean;
  typecheck: "enable" | "try" | "disable";
  debug: boolean;
  debugBundlePath?: string;
  codegen: boolean;
  url: string;
};

export async function runPush(ctx: Context, options: PushOptions) {
  const timeRunPushStarts = performance.now();
  const { configPath, projectConfig } = await readProjectConfig(ctx);
  const origin = options.url;
  const verbose = options.verbose || options.dryRun;
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
    await doCodegen({
      ctx,
      functionsDirectoryPath: functionsDir(configPath, projectConfig),
      typeCheckMode: options.typecheck,
      dryRun: options.dryRun,
      debug: options.debug,
      quiet: true,
    });
    if (verbose) {
      logMessage(ctx, chalk.green("Codegen finished."));
    }
  }

  const timeBundleStarts = performance.now();
  const { config: localConfig, bundledModuleInfos } =
    await configFromProjectConfig(ctx, projectConfig, configPath, verbose);

  if (options.debugBundlePath) {
    const config = configJSON(localConfig, options.adminKey);
    ctx.fs.writeUtf8File(options.debugBundlePath, JSON.stringify(config));
    return;
  }

  const timeSchemaPushStarts = performance.now();
  const { schemaId, schemaState } = await pushSchema(
    ctx,
    origin,
    options.adminKey,
    functionsDir(configPath, localConfig.projectConfig),
    options.dryRun,
  );

  const timeConfigPullStarts = performance.now();
  const remoteConfig = await pullConfig(
    ctx,
    undefined,
    undefined,
    origin,
    options.adminKey,
  );

  changeSpinner(ctx, "Diffing local code and deployment state");
  const diff = diffConfig(remoteConfig, localConfig);
  if (diff === "" && schemaState?.state === "active") {
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
    logMessage(ctx, diff);
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
  };
  await pushConfig(
    ctx,
    localConfig,
    options.adminKey,
    options.url,
    timing,
    schemaId,
    bundledModuleInfos,
  );
}
