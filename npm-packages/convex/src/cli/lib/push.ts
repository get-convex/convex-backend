import chalk from "chalk";
import {
  Context,
  changeSpinner,
  logFailure,
  logMessage,
} from "../../bundler/context.js";
import { doCodegen } from "./codegen.js";
import {
  Config,
  configFromProjectConfig,
  diffConfig,
  pullConfig,
  pushConfig,
  readProjectConfig,
} from "./config.js";
import { pushSchema } from "./indexes.js";
import { typeCheckFunctionsInMode } from "./typecheck.js";
import { ensureHasConvexDependency, functionsDir } from "./utils.js";
import path from "path";

export type PushOptions = {
  adminKey: string;
  verbose: boolean;
  dryRun: boolean;
  typecheck: "enable" | "try" | "disable";
  debug: boolean;
  debugBundlePath?: string;
  codegen: boolean;
  url: string;
  enableComponents: boolean;
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

async function handleDebugBundlePath(
  ctx: Context,
  debugBundleDir: string,
  config: Config,
) {
  if (!ctx.fs.exists(debugBundleDir)) {
    ctx.fs.mkdir(debugBundleDir);
  } else if (!ctx.fs.stat(debugBundleDir).isDirectory()) {
    logFailure(
      ctx,
      `Path \`${debugBundleDir}\` is not a directory. Please choose an empty directory for \`--debug-bundle-path\`.`,
    );
    await ctx.crash(1, "fatal");
  } else if (ctx.fs.listDir(debugBundleDir).length !== 0) {
    logFailure(
      ctx,
      `Directory \`${debugBundleDir}\` is not empty. Please remove it or choose an empty directory for \`--debug-bundle-path\`.`,
    );
    await ctx.crash(1, "fatal");
  }
  ctx.fs.writeUtf8File(
    path.join(debugBundleDir, "fullConfig.json"),
    JSON.stringify(config),
  );
  for (const moduleInfo of config.modules) {
    const trimmedPath = moduleInfo.path.endsWith(".js")
      ? moduleInfo.path.slice(0, moduleInfo.path.length - ".js".length)
      : moduleInfo.path;
    const environmentDir = path.join(debugBundleDir, moduleInfo.environment);
    ctx.fs.mkdir(path.dirname(path.join(environmentDir, `${trimmedPath}.js`)), {
      allowExisting: true,
      recursive: true,
    });
    ctx.fs.writeUtf8File(
      path.join(environmentDir, `${trimmedPath}.js`),
      moduleInfo.source,
    );
    if (moduleInfo.sourceMap !== undefined) {
      ctx.fs.writeUtf8File(
        path.join(environmentDir, `${trimmedPath}.js.map`),
        moduleInfo.sourceMap,
      );
    }
  }
}
