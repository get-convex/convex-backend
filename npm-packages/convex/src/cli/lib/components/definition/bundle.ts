// TODO read this
import path from "path";
import {
  ComponentDirectory,
  ComponentPath,
  buildComponentDirectory,
  isComponentDirectory,
  qualifiedDefinitionPath,
  toComponentPath,
} from "./directoryStructure.js";
import {
  Context,
  logError,
  logMessage,
  logWarning,
} from "../../../../bundler/context.js";
import esbuild, { Metafile, OutputFile, Plugin } from "esbuild";
import chalk from "chalk";
import { DEFINITION_FILENAME } from "../constants.js";
import { createRequire } from "module";
import {
  Bundle,
  bundle,
  bundleSchema,
  entryPointsByEnvironment,
} from "../../../../bundler/index.js";
import {
  AppDefinitionSpecWithoutImpls,
  ComponentDefinitionSpecWithoutImpls,
} from "../../deploy2.js";

/**
 * esbuild plugin to mark component definitions external or return a list of
 * all component definitions.
 *
 * By default this plugin marks component definition files as external,
 * not traversing further.
 *
 * If discover is specified it instead populates the components map,
 * continuing through the entire tree.
 */
function componentPlugin({
  mode,
  rootComponentDirectory,
  verbose,
  ctx,
}: {
  mode: "discover" | "bundle";
  rootComponentDirectory: ComponentDirectory;
  verbose?: boolean;
  ctx: Context;
}): Plugin {
  const components = new Map<string, ComponentDirectory>();
  return {
    name: `convex-${mode === "discover" ? "discover-components" : "bundle-components"}`,
    async setup(build) {
      // This regex can't be really precise since developers could import
      // "component.config", "component.config.js", "component.config.ts", etc.
      build.onResolve(
        // Will it be important when developers write npm package components
        // for these to use a specific entry point or specific file name?
        // I guess we can read the files from the filesystem ourselves!
        // If developers want to import the component definition directly
        // from somewhere else,
        { filter: /.*component.config.*/ },
        async (args) => {
          verbose && logMessage(ctx, "esbuild resolving import:", args);
          if (args.namespace !== "file") {
            verbose && logMessage(ctx, "  Not a file.");
            return;
          }
          if (args.kind === "entry-point") {
            verbose && logMessage(ctx, "  -> Top-level entry-point.");
            const componentDirectory = {
              name: path.basename(path.dirname(args.path)),
              path: args.path,
              definitionPath: path.join(args.path, DEFINITION_FILENAME),
            };
            if (components.get(args.path)) {
              // programmer error
              // eslint-disable-next-line no-restricted-syntax
              throw new Error(
                "why is the entry point component already registered?",
              );
            }
            components.set(args.path, componentDirectory);
            // For an entry point there's no resolution logic to puzzle through
            // since we already have a proper file path.
            // Whether we're bundling or discovering, we're done.
            return;
          }
          const candidates = [args.path];
          const ext = path.extname(args.path);
          if (ext === ".js") {
            candidates.push(args.path.slice(0, -".js".length) + ".ts");
          }
          if (ext !== ".js" && ext !== ".ts") {
            candidates.push(args.path + ".js");
            candidates.push(args.path + ".ts");
          }
          let resolvedPath = undefined;
          for (const candidate of candidates) {
            try {
              // --experimental-import-meta-resolve is required for
              // `import.meta.resolve` so we'll use `require.resolve`
              // until then. Hopefully they aren't too different.
              const require = createRequire(args.resolveDir);

              // Sweet, this does all the Node.js stuff for us!
              resolvedPath = require.resolve(candidate, {
                paths: [args.resolveDir],
              });
              break;
            } catch (e: any) {
              if (e.code === "MODULE_NOT_FOUND") {
                continue;
              }
              // We always catch outside of an esbuild invocation.
              // eslint-disable-next-line no-restricted-syntax
              throw e;
            }
          }
          if (resolvedPath === undefined) {
            // Let `esbuild` handle this itself.
            verbose && logMessage(ctx, `  -> ${args.path} not found.`);
            return;
          }

          const parentDir = path.dirname(resolvedPath);

          let imported = components.get(resolvedPath);
          if (!imported) {
            const isComponent = isComponentDirectory(ctx, parentDir, false);
            if (isComponent.kind !== "ok") {
              verbose && logMessage(ctx, "  -> Not a component:", isComponent);
              return;
            }
            imported = isComponent.component;
            components.set(resolvedPath, imported);
          }

          verbose &&
            logMessage(ctx, "  -> Component import! Recording it.", args.path);

          if (mode === "discover") {
            return {
              path: resolvedPath,
            };
          } else {
            // In bundle mode, transform external imports to use componentPaths:
            // import rateLimiter from "convex_ratelimiter";
            // => import rateLimiter from `_componentDeps/${base64('../node_modules/convex_ratelimiter')}`;

            // A componentPath is path from the root component to the directory
            // of the this component's definition file.
            const componentPath = toComponentPath(
              rootComponentDirectory,
              imported,
            );
            const encodedPath = hackyMapping(componentPath);
            return {
              path: encodedPath,
              external: true,
            };
          }
        },
      );
    },
  };
}

/** The path on the deployment that identifier a component definition. */
function hackyMapping(componentPath: string): string {
  return `./_componentDeps/${Buffer.from(componentPath).toString("base64").replace(/=+$/, "")}`;
}

// Use the metafile to discover the dependency graph in which component
// definitions are nodes.
// If it's cyclic, throw! That's no good!
export async function componentGraph(
  ctx: Context,
  absWorkingDir: string,
  rootComponentDirectory: ComponentDirectory,
  verbose: boolean = true,
): Promise<{
  components: Map<string, ComponentDirectory>;
  dependencyGraph: [ComponentDirectory, ComponentDirectory][];
}> {
  let result;
  try {
    // The discover plugin collects component directories in .components.
    result = await esbuild.build({
      absWorkingDir, // mostly used for formatting error messages
      entryPoints: [qualifiedDefinitionPath(rootComponentDirectory)],
      plugins: [
        componentPlugin({
          ctx,
          mode: "discover",
          verbose,
          rootComponentDirectory,
        }),
      ],
      bundle: true,
      platform: "browser",
      format: "esm",
      target: "esnext",

      sourcemap: "external",
      sourcesContent: false,

      // place output files in memory at their source locations
      write: false,
      outdir: "/",
      outbase: "/",

      minify: true,
      keepNames: true,
      metafile: true,
    });
    await registerEsbuildReads(ctx, absWorkingDir, result.metafile);
  } catch (err: any) {
    logError(ctx, `esbuild failed: ${err}`);
    return await ctx.crash(1, "invalid filesystem data");
  }

  if (result.errors.length) {
    for (const error of result.errors) {
      console.log(chalk.red(`esbuild error: ${error.text}`));
    }
    return await ctx.crash(1, "invalid filesystem data");
  }
  // TODO we're going to end up printing these warnings twice right now
  for (const warning of result.warnings) {
    console.log(chalk.yellow(`esbuild warning: ${warning.text}`));
  }
  return await findComponentDependencies(ctx, result.metafile);
}

/**
 * Get dependencies of a ComponenDirectory as ComponentPaths.
 *
 * Component paths are paths relative to the root component.
 */
export function getDeps(
  rootComponent: ComponentDirectory,
  dependencyGraph: [ComponentDirectory, ComponentDirectory][],
  definitionPath: string,
): ComponentPath[] {
  return dependencyGraph
    .filter(
      ([importer, _imported]) => importer.definitionPath === definitionPath,
    )
    .map(([_importer, imported]) => toComponentPath(rootComponent, imported));
}

/**
 * The returned dependency graph is an array of tuples of [importer, imported]
 *
 * This doesn't work on just any esbuild metafile; it has to be run with
 * the component esbuilt plugin run in "discover" mode so that it won't modify anything.
 */
async function findComponentDependencies(
  ctx: Context,
  metafile: Metafile,
): Promise<{
  components: Map<string, ComponentDirectory>;
  dependencyGraph: [ComponentDirectory, ComponentDirectory][];
}> {
  // The esbuild metafile has inputs as relative paths from cwd
  // but the imports for each input are absolute paths or unqualified
  // paths when marked external.
  const { inputs } = metafile;
  // TODO compress these inputs so that everything that is not a component.config.ts
  // or app.config.ts has its dependencies compacted down.
  // Until then we just let these missing links slip through.
  const componentInputs = Object.keys(inputs).filter((path) =>
    path.includes(".config."),
  );

  // Absolute path doesn't appear to be necessary here since only inputs marked
  // external get transformed to an absolute path but it's not clear what's an
  // esbuild implementation detail in the metafile or which settings change this.
  const componentsByAbsPath = new Map<string, ComponentDirectory>();
  for (const inputPath of componentInputs) {
    const importer = await buildComponentDirectory(ctx, inputPath);
    componentsByAbsPath.set(path.resolve(inputPath), importer);
  }
  const dependencyGraph: [ComponentDirectory, ComponentDirectory][] = [];
  for (const inputPath of componentInputs) {
    const importer = componentsByAbsPath.get(path.resolve(inputPath))!;
    const { imports } = inputs[inputPath];
    const componentImports = imports.filter((imp) =>
      imp.path.includes(".config."),
    );
    for (const importPath of componentImports.map((dep) => dep.path)) {
      const imported = componentsByAbsPath.get(path.resolve(importPath));
      if (!imported) {
        logError(
          ctx,
          `Didn't find ${path.resolve(importPath)} in ${[...componentsByAbsPath.keys()].toString()}`,
        );
        return await ctx.crash(1, "fatal");
      }
      dependencyGraph.push([importer, imported]);
    }
  }

  const components = new Map<string, ComponentDirectory>();
  for (const directory of componentsByAbsPath.values()) {
    components.set(directory.path, directory);
  }
  return { components, dependencyGraph };
}

/**
 * Bundle definitions listed in directories. An app.config.ts must exist
 * in the absWorkingDir.
 * If a directory linked to is not listed then there will be external links
 * with no corresponding definition bundle.
 * That could be made to throw an error but maybe those are already available
 * on the Convex definition filesystem somehow, e.g. builtin components.
 */
export async function bundleDefinitions(
  ctx: Context,
  absWorkingDir: string,
  dependencyGraph: [ComponentDirectory, ComponentDirectory][],
  rootComponentDirectory: ComponentDirectory,
  componentDirectories: ComponentDirectory[],
  verbose: boolean = false,
): Promise<{
  appDefinitionSpecWithoutImpls: AppDefinitionSpecWithoutImpls;
  componentDefinitionSpecsWithoutImpls: ComponentDefinitionSpecWithoutImpls[];
}> {
  let result;
  try {
    // TODO These should all come from ../../../bundler/index.js, at least helpers.
    result = await esbuild.build({
      absWorkingDir: absWorkingDir,
      entryPoints: componentDirectories.map((dir) =>
        qualifiedDefinitionPath(dir),
      ),
      plugins: [
        componentPlugin({
          ctx,
          mode: "bundle",
          verbose,
          rootComponentDirectory,
        }),
      ],
      bundle: true,
      platform: "browser",
      format: "esm",
      target: "esnext",

      // false is the default for splitting.
      // It's simpler to evaluate these on the server when we don't need a whole
      // filesystem. Enabled this for speed once the server supports it.
      splitting: false,

      // whatever
      sourcemap: false,
      sourcesContent: false,

      // place output files in memory at their source locations
      write: false,
      outdir: "/",
      outbase: "/",

      // debugging over bundle size for now, change later
      minify: false,
      keepNames: true,

      // Either we trust dependencies passed in or we build our own.
      // Better to be consistent here?
      metafile: true,
    });
    await registerEsbuildReads(ctx, absWorkingDir, result.metafile);
  } catch (err: any) {
    logError(ctx, `esbuild failed: ${err}`);
    return await ctx.crash(1, "invalid filesystem data");
  }
  // TODO In theory we should get exactly the same errors here as we did the first time. Do something about that.
  // TODO abstract out this esbuild wrapper stuff now that it's in two places
  if (result.errors.length) {
    for (const error of result.errors) {
      console.log(chalk.red(`esbuild error: ${error.text}`));
    }
    return await ctx.crash(1, "invalid filesystem data");
  }
  for (const warning of result.warnings) {
    console.log(chalk.yellow(`esbuild warning: ${warning.text}`));
  }

  const outputs: {
    outputJs: OutputFile;
    outputJsMap?: OutputFile;
    directory: ComponentDirectory;
  }[] = [];
  for (const directory of componentDirectories) {
    const absInput = path.resolve(absWorkingDir, directory.definitionPath);
    const expectedOutputJs =
      absInput.slice(0, absInput.lastIndexOf(".")) + ".js";
    const expectedOutputMap =
      absInput.slice(0, absInput.lastIndexOf(".")) + ".js.map";
    const outputJs = result.outputFiles.filter(
      (outputFile) => outputFile.path === expectedOutputJs,
    )[0];
    if (!outputJs) {
      logError(
        ctx,
        `no JS found matching ${expectedOutputJs} in ${result.outputFiles.map((x) => x.path).toString()}`,
      );
      return await ctx.crash(1, "fatal");
    }
    const outputJsMap = result.outputFiles.filter(
      (outputFile) => outputFile.path === expectedOutputMap,
    )[0];
    outputs.push({
      outputJs,
      outputJsMap,
      directory,
    });
  }

  const [appBundle] = outputs.filter(
    (out) => out.directory.path === rootComponentDirectory.path,
  );
  const componentBundles = outputs.filter(
    (out) => out.directory.path !== rootComponentDirectory.path,
  );

  const componentDefinitionSpecsWithoutImpls = componentBundles.map(
    ({ directory, outputJs, outputJsMap }) => ({
      definitionPath: directory.path,
      definition: {
        path: path.relative(directory.path, outputJs.path),
        source: outputJs.text,
        sourceMap: outputJsMap?.text,
        environment: "isolate" as const,
      },
      dependencies: getDeps(
        rootComponentDirectory,
        dependencyGraph,
        directory.definitionPath,
      ),
    }),
  );
  const appDeps = getDeps(
    rootComponentDirectory,
    dependencyGraph,
    appBundle.directory.definitionPath,
  );
  const appDefinitionSpecWithoutImpls = {
    definition: {
      path: path.relative(rootComponentDirectory.path, appBundle.outputJs.path),
      source: appBundle.outputJs.text,
      sourceMap: appBundle.outputJsMap?.text,
      environment: "isolate" as const,
    },
    dependencies: appDeps,
  };
  return {
    appDefinitionSpecWithoutImpls,
    componentDefinitionSpecsWithoutImpls,
  };
}

export async function bundleImplementations(
  ctx: Context,
  rootComponentDirectory: ComponentDirectory,
  componentDirectories: ComponentDirectory[],
  verbose: boolean = false,
): Promise<{
  appImplementation: {
    schema: Bundle;
    functions: Bundle[];
  };
  componentImplementations: {
    schema: Bundle;
    functions: Bundle[];
    definitionPath: string;
  }[];
}> {
  let appImplementation;
  const componentImplementations = [];

  let isRoot = true;
  for (const directory of [rootComponentDirectory, ...componentDirectories]) {
    const resolvedPath = path.resolve(
      rootComponentDirectory.path,
      directory.path,
    );
    const schema = (await bundleSchema(ctx, resolvedPath))[0] || null;

    // TODO figure out how this logic applies to non convex directories
    // for components not defined in one.
    const entryPoints = await entryPointsByEnvironment(
      ctx,
      resolvedPath,
      verbose,
    );
    const convexResult: {
      modules: Bundle[];
      externalDependencies: Map<string, string>;
      bundledModuleNames: Set<string>;
    } = await bundle(ctx, resolvedPath, entryPoints.isolate, true, "browser");

    if (convexResult.externalDependencies.size !== 0) {
      logError(ctx, "TODO external deps");
      return await ctx.crash(1, "fatal");
    }

    // TODO Node compilation (how will Lambdas even work?)
    const _nodeResult = {
      bundles: [],
      externalDependencies: new Map(),
      bundledModuleNames: new Set(),
    };

    const functions = convexResult.modules;
    if (isRoot) {
      appImplementation = {
        schema,
        functions,
      };
    } else {
      componentImplementations.push({
        // these needs to be a componentPath when sent to the server
        definitionPath: toComponentPath(rootComponentDirectory, directory),
        schema,
        functions,
      });
    }
    isRoot = false;
  }

  if (!appImplementation) {
    // TODO should be enforced earlier
    logError(ctx, "No app implementation found");
    return await ctx.crash(1, "fatal");
  }

  return { appImplementation, componentImplementations };
}

// TODO ensure this isn't broken with changes to workingdir location
async function registerEsbuildReads(
  ctx: Context,
  absWorkingDir: string,
  metafile: Metafile,
) {
  for (const [relPath, input] of Object.entries(metafile.inputs)) {
    // TODO: esbuild outputs paths prefixed with "(disabled)"" when bundling our internal
    // udf-system package. The files do actually exist locally, though.
    if (
      relPath.indexOf("(disabled):") !== -1 ||
      relPath.startsWith("wasm-binary:") ||
      relPath.startsWith("wasm-stub:")
    ) {
      continue;
    }
    const absPath = path.resolve(absWorkingDir, relPath);
    const st = ctx.fs.stat(absPath);
    if (st.size !== input.bytes) {
      logWarning(
        ctx,
        `Bundled file ${absPath} changed right after esbuild invocation`,
      );
      // Consider this a transient error so we'll try again and hopefully
      // no files change right after esbuild next time.
      return await ctx.crash(1, "transient");
    }
    ctx.fs.registerPath(absPath, st);
  }
}
