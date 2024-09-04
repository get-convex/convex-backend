import path from "path";
import {
  Context,
  changeSpinner,
  logFinishedStep,
  logMessage,
} from "../../bundler/context.js";
import {
  ProjectConfig,
  configFromProjectConfig,
  getFunctionsDirectoryPath,
  readProjectConfig,
} from "./config.js";
import { finishPush, startPush, waitForSchema } from "./deploy2.js";
import { version } from "../version.js";
import { PushOptions, runNonComponentsPush } from "./push.js";
import { ensureHasConvexDependency, functionsDir } from "./utils/utils.js";
import {
  bundleDefinitions,
  bundleImplementations,
  componentGraph,
} from "./components/definition/bundle.js";
import { isComponentDirectory } from "./components/definition/directoryStructure.js";
import {
  doFinalComponentCodegen,
  doInitialComponentCodegen,
  CodegenOptions,
  doInitCodegen,
  doCodegen,
} from "./codegen.js";
import {
  AppDefinitionConfig,
  ComponentDefinitionConfig,
} from "./deployApi/definitionConfig.js";
import { typeCheckFunctionsInMode, TypeCheckMode } from "./typecheck.js";
import { withTmpDir } from "../../bundler/fs.js";
import { ROOT_DEFINITION_FILENAME } from "./components/constants.js";
import { handleDebugBundlePath } from "./debugBundlePath.js";
import chalk from "chalk";
import { StartPushResponse } from "./deployApi/startPush.js";
import {
  deploymentSelectionFromOptions,
  fetchDeploymentCredentialsProvisionProd,
} from "./api.js";
import {
  FinishPushDiff,
  DeveloperIndexConfig,
} from "./deployApi/finishPush.js";

export async function runCodegen(ctx: Context, options: CodegenOptions) {
  // This also ensures the current directory is the project root.
  await ensureHasConvexDependency(ctx, "codegen");

  const { configPath, projectConfig } = await readProjectConfig(ctx);
  const functionsDirectoryPath = functionsDir(configPath, projectConfig);
  const componentRootPath = path.resolve(
    path.join(functionsDirectoryPath, ROOT_DEFINITION_FILENAME),
  );
  if (ctx.fs.exists(componentRootPath)) {
    const deploymentSelection = deploymentSelectionFromOptions(options);
    const credentials = await fetchDeploymentCredentialsProvisionProd(
      ctx,
      deploymentSelection,
    );

    await startComponentsPushAndCodegen(ctx, projectConfig, configPath, {
      ...options,
      ...credentials,
      generateCommonJSApi: options.commonjs,
      verbose: options.dryRun,
    });
  } else {
    if (options.init) {
      await doInitCodegen(ctx, functionsDirectoryPath, false, {
        dryRun: options.dryRun,
        debug: options.debug,
      });
    }

    if (options.typecheck !== "disable") {
      logMessage(ctx, chalk.gray("Running TypeScript typecheck…"));
    }

    await doCodegen(ctx, functionsDirectoryPath, options.typecheck, {
      dryRun: options.dryRun,
      debug: options.debug,
      generateCommonJSApi: options.commonjs,
    });
  }
}

export async function runPush(ctx: Context, options: PushOptions) {
  const { configPath, projectConfig } = await readProjectConfig(ctx);
  const convexDir = functionsDir(configPath, projectConfig);
  const componentRootPath = path.resolve(
    path.join(convexDir, ROOT_DEFINITION_FILENAME),
  );
  if (ctx.fs.exists(componentRootPath)) {
    await runComponentsPush(ctx, options, configPath, projectConfig);
  } else {
    await runNonComponentsPush(ctx, options, configPath, projectConfig);
  }
}

async function startComponentsPushAndCodegen(
  ctx: Context,
  projectConfig: ProjectConfig,
  configPath: string,
  options: {
    typecheck: TypeCheckMode;
    adminKey: string;
    url: string;
    verbose: boolean;
    debugBundlePath?: string;
    dryRun: boolean;
    generateCommonJSApi?: boolean;
    debug: boolean;
    writePushRequest?: string;
  },
): Promise<StartPushResponse | null> {
  const verbose = options.verbose || options.dryRun;
  const convexDir = await getFunctionsDirectoryPath(ctx);

  // '.' means use the process current working directory, it's the default behavior.
  // Spelling it out here to be explicit for a future where this code can run
  // from other directories.
  // In esbuild the working directory is used to print error messages and resolving
  // relatives paths passed to it. It generally doesn't matter for resolving imports,
  // imports are resolved from the file where they are written.
  const absWorkingDir = path.resolve(".");
  const isComponent = isComponentDirectory(ctx, convexDir, true);
  if (isComponent.kind === "err") {
    return await ctx.crash({
      exitCode: 1,
      errorType: "invalid filesystem data",
      printedMessage: `Invalid component root directory (${isComponent.why}): ${convexDir}`,
    });
  }
  const rootComponent = isComponent.component;

  changeSpinner(ctx, "Finding component definitions...");
  // Create a list of relevant component directories. These are just for knowing
  // while directories to bundle in bundleDefinitions and bundleImplementations.
  // This produces a bundle in memory as a side effect but it's thrown away.
  //
  // This is the very first time we traverse the component graph.
  // We're just traversing to discover
  const { components, dependencyGraph } = await componentGraph(
    ctx,
    absWorkingDir,
    rootComponent,
    verbose,
  );

  changeSpinner(ctx, "Generating server code...");
  await withTmpDir(async (tmpDir) => {
    await doInitialComponentCodegen(ctx, tmpDir, rootComponent, options);
    for (const directory of components.values()) {
      await doInitialComponentCodegen(ctx, tmpDir, directory, options);
    }
  });

  changeSpinner(ctx, "Bundling component definitions...");
  // This bundles everything but the actual function definitions
  const {
    appDefinitionSpecWithoutImpls,
    componentDefinitionSpecsWithoutImpls,
  } = await bundleDefinitions(
    ctx,
    absWorkingDir,
    dependencyGraph,
    rootComponent,
    // Note that this *includes* the root component.
    [...components.values()],
  );

  changeSpinner(ctx, "Bundling component schemas and implementations...");
  const { appImplementation, componentImplementations } =
    await bundleImplementations(
      ctx,
      rootComponent,
      [...components.values()],
      projectConfig.node.externalPackages,
      verbose,
    );
  if (options.debugBundlePath) {
    const { config: localConfig } = await configFromProjectConfig(
      ctx,
      projectConfig,
      configPath,
      verbose,
    );
    // TODO(ENG-6972): Actually write the bundles for components.
    await handleDebugBundlePath(ctx, options.debugBundlePath, localConfig);
    logMessage(
      ctx,
      `Wrote bundle and metadata for modules in the root to ${options.debugBundlePath}. Skipping rest of push.`,
    );
    return null;
  }

  // We're just using the version this CLI is running with for now.
  // This could be different than the version of `convex` the app runs with
  // if the CLI is installed globally.
  // TODO: This should be the version of the `convex` package used by each
  // component, and may be different for each component.
  const udfServerVersion = version;

  const appDefinition: AppDefinitionConfig = {
    ...appDefinitionSpecWithoutImpls,
    ...appImplementation,
    udfServerVersion,
  };

  const componentDefinitions: ComponentDefinitionConfig[] = [];
  for (const componentDefinition of componentDefinitionSpecsWithoutImpls) {
    const impl = componentImplementations.filter(
      (impl) => impl.definitionPath === componentDefinition.definitionPath,
    )[0];
    if (!impl) {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: `missing! couldn't find ${componentDefinition.definitionPath} in ${componentImplementations.map((impl) => impl.definitionPath).toString()}`,
      });
    }
    componentDefinitions.push({
      ...componentDefinition,
      ...impl,
      udfServerVersion,
    });
  }
  const startPushRequest = {
    adminKey: options.adminKey,
    dryRun: false,
    functions: projectConfig.functions,
    appDefinition,
    componentDefinitions,
    nodeDependencies: appImplementation.externalNodeDependencies,
  };
  if (options.writePushRequest) {
    const pushRequestPath = path.resolve(options.writePushRequest);
    ctx.fs.writeUtf8File(
      `${pushRequestPath}.json`,
      JSON.stringify(startPushRequest),
    );
    return null;
  }

  changeSpinner(ctx, "Uploading functions to Convex...");
  const startPushResponse = await startPush(
    ctx,
    options.url,
    startPushRequest,
    verbose,
  );

  verbose && console.log("startPush:");
  verbose && console.dir(startPushResponse, { depth: null });

  changeSpinner(ctx, "Generating TypeScript bindings...");
  await withTmpDir(async (tmpDir) => {
    await doFinalComponentCodegen(
      ctx,
      tmpDir,
      rootComponent,
      rootComponent,
      startPushResponse,
      options,
    );
    for (const directory of components.values()) {
      await doFinalComponentCodegen(
        ctx,
        tmpDir,
        rootComponent,
        directory,
        startPushResponse,
        options,
      );
    }
  });

  changeSpinner(ctx, "Running TypeScript...");
  await typeCheckFunctionsInMode(ctx, options.typecheck, rootComponent.path);
  for (const directory of components.values()) {
    await typeCheckFunctionsInMode(ctx, options.typecheck, directory.path);
  }

  return startPushResponse;
}

export async function runComponentsPush(
  ctx: Context,
  options: PushOptions,
  configPath: string,
  projectConfig: ProjectConfig,
) {
  await ensureHasConvexDependency(ctx, "push");

  if (options.dryRun) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: "dryRun not allowed yet",
    });
  }

  const startPushResponse = await startComponentsPushAndCodegen(
    ctx,
    projectConfig,
    configPath,
    options,
  );
  if (!startPushResponse) {
    return;
  }

  await waitForSchema(ctx, options.adminKey, options.url, startPushResponse);

  const finishPushResponse = await finishPush(
    ctx,
    options.adminKey,
    options.url,
    startPushResponse,
  );
  printDiff(ctx, finishPushResponse, options);
}

function printDiff(
  ctx: Context,
  finishPushResponse: FinishPushDiff,
  opts: { verbose: boolean; dryRun: boolean },
) {
  if (opts.verbose) {
    const diffString = JSON.stringify(finishPushResponse, null, 2);
    logMessage(ctx, diffString);
    return;
  }
  const { componentDiffs } = finishPushResponse;

  // Print out index diffs for the root component.
  let rootDiff = componentDiffs[""];
  if (rootDiff) {
    if (rootDiff.indexDiff.removed_indexes.length > 0) {
      let msg = `${opts.dryRun ? "Would delete" : "Deleted"} table indexes:\n`;
      for (let i = 0; i < rootDiff.indexDiff.removed_indexes.length; i++) {
        const index = rootDiff.indexDiff.removed_indexes[i];
        if (i > 0) {
          msg += "\n";
        }
        msg += `  [-] ${formatIndex(index)}`;
      }
      logFinishedStep(ctx, msg);
    }
    if (rootDiff.indexDiff.added_indexes.length > 0) {
      let msg = `${opts.dryRun ? "Would add" : "Added"} table indexes:\n`;
      for (let i = 0; i < rootDiff.indexDiff.added_indexes.length; i++) {
        const index = rootDiff.indexDiff.added_indexes[i];
        if (i > 0) {
          msg += "\n";
        }
        msg += `  [+] ${formatIndex(index)}`;
      }
      logFinishedStep(ctx, msg);
    }
  }

  // Only show component level diffs for other components.
  for (const [componentPath, componentDiff] of Object.entries(componentDiffs)) {
    if (componentPath === "") {
      continue;
    }
    if (componentDiff.diffType.type === "create") {
      logFinishedStep(ctx, `Installed component ${componentPath}.`);
    }
    if (componentDiff.diffType.type === "unmount") {
      logFinishedStep(ctx, `Unmounted component ${componentPath}.`);
    }
  }
}

function formatIndex(index: DeveloperIndexConfig) {
  return `${index.name}`;
}
