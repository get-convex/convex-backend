import path from "path";
import { Context, changeSpinner, logError } from "../../bundler/context.js";
import {
  ProjectConfig,
  configFromProjectConfig,
  readProjectConfig,
} from "./config.js";
import { finishPush, startPush, waitForSchema } from "./deploy2.js";
import { version } from "../version.js";
import { PushOptions, runNonComponentsPush } from "./push.js";
import { ensureHasConvexDependency, functionsDir } from "./utils.js";
import {
  bundleDefinitions,
  bundleImplementations,
  componentGraph,
} from "./components/definition/bundle.js";
import { isComponentDirectory } from "./components/definition/directoryStructure.js";
import {
  doFinalComponentCodegen,
  doInitialComponentCodegen,
} from "./codegen.js";
import {
  AppDefinitionConfig,
  ComponentDefinitionConfig,
} from "./deployApi/definitionConfig.js";
import { typeCheckFunctionsInMode } from "./typecheck.js";
import { withTmpDir } from "../../bundler/fs.js";
import { ROOT_DEFINITION_FILENAME } from "./components/constants.js";

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

export async function runComponentsPush(
  ctx: Context,
  options: PushOptions,
  configPath: string,
  projectConfig: ProjectConfig,
) {
  const verbose = options.verbose || options.dryRun;
  await ensureHasConvexDependency(ctx, "push");

  if (options.dryRun) {
    logError(ctx, "dryRun not allowed yet");
    await ctx.crash(1, "fatal");
  }
  if (options.debugBundlePath) {
    logError(ctx, "debugBundlePath not allowed yet");
    await ctx.crash(1, "fatal");
  }

  const convexDir = functionsDir(configPath, projectConfig);

  // '.' means use the process current working directory, it's the default behavior.
  // Spelling it out here to be explicit for a future where this code can run
  // from other directories.
  // In esbuild the working directory is used to print error messages and resolving
  // relatives paths passed to it. It generally doesn't matter for resolving imports,
  // imports are resolved from the file where they are written.
  const absWorkingDir = path.resolve(".");
  const isComponent = isComponentDirectory(ctx, convexDir, true);
  if (isComponent.kind === "err") {
    logError(
      ctx,
      `Invalid component root directory (${isComponent.why}): ${convexDir}`,
    );
    return await ctx.crash(1, "invalid filesystem data");
  }
  const rootComponent = isComponent.component;

  changeSpinner(ctx, "Traversing component definitions...");
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

  const { config: localConfig } = await configFromProjectConfig(
    ctx,
    projectConfig,
    configPath,
    verbose,
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

  // We're just using the version this CLI is running with for now.
  // This could be different than the version of `convex` the app runs with
  // if the CLI is installed globally.
  // TODO: This should be the version of the `convex` package used by each
  // component, and may be different for each component.
  const udfServerVersion = version;

  const appDefinition: AppDefinitionConfig = {
    ...appDefinitionSpecWithoutImpls,
    auth: localConfig.authConfig || null,
    ...appImplementation,
    udfServerVersion,
  };

  const componentDefinitions: ComponentDefinitionConfig[] = [];
  for (const componentDefinition of componentDefinitionSpecsWithoutImpls) {
    const impl = componentImplementations.filter(
      (impl) =>
        // convert from ComponentPath
        path.resolve(rootComponent.path, impl.definitionPath) ===
        componentDefinition.definitionPath,
    )[0];
    if (!impl) {
      console.log(
        `missing! couldn't find ${componentDefinition.definitionPath} in ${componentImplementations.map((impl) => path.resolve(rootComponent.path, impl.definitionPath)).toString()}`,
      );
      return await ctx.crash(1, "fatal");
    }
    componentDefinitions.push({
      ...componentDefinition,
      ...impl,
      udfServerVersion,
    });
  }

  const startPushResponse = await startPush(
    ctx,
    options.url,
    {
      adminKey: options.adminKey,
      dryRun: false,
      functions: projectConfig.functions,
      appDefinition,
      componentDefinitions,
      nodeDependencies: appImplementation.externalNodeDependencies,
    },
    verbose,
  );

  verbose && console.log("startPush:");
  verbose && console.dir(startPushResponse, { depth: null });

  changeSpinner(ctx, "Finalizing code generation...");
  await withTmpDir(async (tmpDir) => {
    await doFinalComponentCodegen(
      ctx,
      tmpDir,
      rootComponent,
      rootComponent,
      startPushResponse,
    );
    for (const directory of components.values()) {
      await doFinalComponentCodegen(
        ctx,
        tmpDir,
        rootComponent,
        directory,
        startPushResponse,
      );
    }
  });

  changeSpinner(ctx, "Running TypeScript...");
  await typeCheckFunctionsInMode(ctx, options.typecheck, rootComponent.path);
  for (const directory of components.values()) {
    await typeCheckFunctionsInMode(ctx, options.typecheck, directory.path);
  }

  changeSpinner(ctx, "Waiting for schema...");
  await waitForSchema(ctx, options.adminKey, options.url, startPushResponse);

  const finishPushResponse = await finishPush(
    ctx,
    options.adminKey,
    options.url,
    startPushResponse,
  );
  verbose && console.log("finishPush:", finishPushResponse);
}
