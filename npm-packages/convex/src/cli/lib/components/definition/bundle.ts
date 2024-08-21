import path from "path";
import {
  ComponentDirectory,
  ComponentDefinitionPath,
  buildComponentDirectory,
  isComponentDirectory,
  qualifiedDefinitionPath,
  toComponentDefinitionPath,
} from "./directoryStructure.js";
import {
  Context,
  logMessage,
  logWarning,
  showSpinner,
} from "../../../../bundler/context.js";
import esbuild, { BuildOptions, Metafile, OutputFile, Plugin } from "esbuild";
import chalk from "chalk";
import { createRequire } from "module";
import {
  AppDefinitionSpecWithoutImpls,
  ComponentDefinitionSpecWithoutImpls,
} from "../../config.js";
import {
  Bundle,
  bundle,
  bundleAuthConfig,
  bundleSchema,
  entryPointsByEnvironment,
} from "../../../../bundler/index.js";
import { NodeDependency } from "../../deployApi/modules.js";

/**
 * An esbuild plugin to mark component definitions external or return a list of
 * all component definitions.
 *
 * By default this plugin runs in "bundle" mode and marks all imported component
 * definition files as external, not traversing further.
 *
 * If "discover" mode is specified it traverses the entire tree.
 */
function componentPlugin({
  mode = "bundle",
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
      // "convex.config", "convex.config.js", "convex.config.ts", etc.
      build.onResolve({ filter: /.*convex.config.*/ }, async (args) => {
        verbose && logMessage(ctx, "esbuild resolving import:", args);
        if (args.namespace !== "file") {
          verbose && logMessage(ctx, "  Not a file.");
          return;
        }
        if (args.kind === "entry-point") {
          verbose && logMessage(ctx, "  -> Top-level entry-point.");
          const componentDirectory = await buildComponentDirectory(
            ctx,
            path.resolve(args.path),
          );

          // No attempt to resolve args.path is made for entry points so they
          // must be relative or absolute file paths, not npm packages.
          // Whether we're bundling or discovering, we're done.
          if (components.get(args.path)) {
            // We always invoke esbuild in a try/catch.
            // eslint-disable-next-line no-restricted-syntax
            throw new Error(
              `Entry point component "${args.path}" already registered.`,
            );
          }
          components.set(args.path, componentDirectory);
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
            resolvedPath = require.resolve(candidate, {
              paths: [args.resolveDir],
            });
            break;
          } catch (e: any) {
            if (e.code === "MODULE_NOT_FOUND") {
              continue;
            }
            // We always invoke esbuild in a try/catch.
            // eslint-disable-next-line no-restricted-syntax
            throw e;
          }
        }
        if (resolvedPath === undefined) {
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
          const componentPath = toComponentDefinitionPath(
            rootComponentDirectory,
            imported,
          );
          const encodedPath = hackyMapping(componentPath);
          return {
            path: encodedPath,
            external: true,
          };
        }
      });
    },
  };
}

/** The path on the deployment that identifier a component definition. */
function hackyMapping(componentPath: ComponentDefinitionPath): string {
  return `./_componentDeps/${Buffer.from(componentPath).toString("base64").replace(/=+$/, "")}`;
}

// Share configuration between the component definition discovery and bundling passes.
const SHARED_ESBUILD_OPTIONS = {
  bundle: true,
  platform: "browser",
  format: "esm",
  target: "esnext",

  // false is the default for splitting.
  // It's simpler to evaluate these on the server when we don't need a whole
  // filesystem. Enabled this for speed once the server supports it.
  splitting: false,

  // place output files in memory at their source locations
  write: false,
  outdir: path.parse(process.cwd()).root,
  outbase: path.parse(process.cwd()).root,

  minify: true,
  keepNames: true,

  metafile: true,
} as const satisfies BuildOptions;

// Use the esbuild metafile to discover the dependency graph in which component
// definitions are nodes.
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
    result = await esbuild.build({
      absWorkingDir, // This is mostly useful for formatting error messages.
      entryPoints: [qualifiedDefinitionPath(rootComponentDirectory)],
      plugins: [
        componentPlugin({
          ctx,
          mode: "discover",
          verbose,
          rootComponentDirectory,
        }),
      ],
      sourcemap: "external",
      sourcesContent: false,

      ...SHARED_ESBUILD_OPTIONS,
    });
    await registerEsbuildReads(ctx, absWorkingDir, result.metafile);
  } catch (err: any) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "invalid filesystem data",
      printedMessage: `esbuild failed: ${err}`,
    });
  }

  if (result.errors.length) {
    const message = result.errors.map((error) => error.text).join("\n");
    return await ctx.crash({
      exitCode: 1,
      errorType: "invalid filesystem data",
      printedMessage: message,
    });
  }
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
): ComponentDefinitionPath[] {
  return dependencyGraph
    .filter(
      ([importer, _imported]) => importer.definitionPath === definitionPath,
    )
    .map(([_importer, imported]) =>
      toComponentDefinitionPath(rootComponent, imported),
    );
}

/**
 * The returned dependency graph is an array of tuples of [importer, imported]
 *
 * This doesn't work on just any esbuild metafile because it assumes input
 * imports have not been transformed. We run it on the metafile produced by
 * the esbuild invocation that uses the component plugin in "discover" mode.
 */
async function findComponentDependencies(
  ctx: Context,
  metafile: Metafile,
): Promise<{
  components: Map<string, ComponentDirectory>;
  dependencyGraph: [ComponentDirectory, ComponentDirectory][];
}> {
  const { inputs } = metafile;
  // This filter means we only supports *direct imports* of component definitions
  // from other component definitions.
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
        return await ctx.crash({
          exitCode: 1,
          errorType: "invalid filesystem data",
          printedMessage: `Didn't find ${path.resolve(importPath)} in ${[...componentsByAbsPath.keys()].toString()}`,
        });
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

// NB: If a directory linked to is not a member of the passed
// componentDirectories array then there will be external links
// with no corresponding definition bundle.
// That could be made to throw an error but maybe those are already available
// on the Convex definition filesystem somehow, e.g. builtin components.
/** Bundle the component definitions listed. */
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
    result = await esbuild.build({
      absWorkingDir,
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
      sourcemap: false, // we're just building a deps map
      ...SHARED_ESBUILD_OPTIONS,
    });
    await registerEsbuildReads(ctx, absWorkingDir, result.metafile);
  } catch (err: any) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "invalid filesystem data",
      printedMessage: `esbuild failed: ${err}`,
    });
  }

  if (result.errors.length) {
    const message = result.errors.map((error) => error.text).join("\n");
    return await ctx.crash({
      exitCode: 1,
      errorType: "invalid filesystem data",
      printedMessage: message,
    });
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
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: `no JS found matching ${expectedOutputJs} in ${result.outputFiles.map((x) => x.path).toString()}`,
      });
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

  const appBundles = outputs.filter(
    (out) => out.directory.path === rootComponentDirectory.path,
  );
  if (appBundles.length !== 1) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: "found wrong number of app bundles",
    });
  }
  const appBundle = appBundles[0];
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
  nodeExternalPackages: string[],
  verbose: boolean = false,
): Promise<{
  appImplementation: {
    schema: Bundle | null;
    functions: Bundle[];
    externalNodeDependencies: NodeDependency[];
  };
  componentImplementations: {
    schema: Bundle | null;
    functions: Bundle[];
    definitionPath: ComponentDefinitionPath;
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
    let schema;
    if (!ctx.fs.exists(path.resolve(resolvedPath, "schema.ts"))) {
      schema = null;
    } else {
      schema = (await bundleSchema(ctx, resolvedPath))[0] || null;
    }

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
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: "external dependencies not supported",
      });
    }
    const functions = convexResult.modules;
    if (isRoot) {
      if (verbose) {
        showSpinner(ctx, "Bundling modules for Node.js runtime...");
      }
      const nodeResult: {
        modules: Bundle[];
        externalDependencies: Map<string, string>;
        bundledModuleNames: Set<string>;
      } = await bundle(
        ctx,
        resolvedPath,
        entryPoints.node,
        true,
        "node",
        path.join("_deps", "node"),
        nodeExternalPackages,
      );

      const externalNodeDependencies: NodeDependency[] = [];
      for (const [
        moduleName,
        moduleVersion,
      ] of nodeResult.externalDependencies) {
        externalNodeDependencies.push({
          name: moduleName,
          version: moduleVersion,
        });
      }
      const authBundle = await bundleAuthConfig(ctx, resolvedPath);
      appImplementation = {
        schema,
        functions: functions.concat(nodeResult.modules).concat(authBundle),
        externalNodeDependencies,
      };
    } else {
      componentImplementations.push({
        // these needs to be a componentPath when sent to the server
        definitionPath: toComponentDefinitionPath(
          rootComponentDirectory,
          directory,
        ),
        schema,
        functions,
      });
    }
    isRoot = false;
  }

  if (!appImplementation) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: "No app implementation found",
    });
  }

  return { appImplementation, componentImplementations };
}

async function registerEsbuildReads(
  ctx: Context,
  absWorkingDir: string,
  metafile: Metafile,
) {
  for (const [relPath, input] of Object.entries(metafile.inputs)) {
    if (
      // We rewrite these files so this integrity check isn't useful.
      path.basename(relPath).includes("convex.config") ||
      // TODO: esbuild outputs paths prefixed with "(disabled)" when bundling our internal
      // udf-system package. The files do actually exist locally, though.
      relPath.indexOf("(disabled):") !== -1 ||
      relPath.startsWith("wasm-binary:") ||
      relPath.startsWith("wasm-stub:")
    ) {
      continue;
    }
    const absPath = path.resolve(absWorkingDir, relPath);
    const st = ctx.fs.stat(absPath);
    if (st.size !== input.bytes) {
      // Consider this a transient error so we'll try again and hopefully
      // no files change right after esbuild next time.
      logWarning(
        ctx,
        `Bundled file ${absPath} changed right after esbuild invocation`,
      );
      return await ctx.crash({
        exitCode: 1,
        errorType: "transient",
        printedMessage: null,
      });
    }
    ctx.fs.registerPath(absPath, st);
  }
}
