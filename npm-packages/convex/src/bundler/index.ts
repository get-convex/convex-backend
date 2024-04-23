import path from "path";
import chalk from "chalk";
import esbuild from "esbuild";
import { parse as parseAST } from "@babel/parser";
import { Identifier, ImportSpecifier } from "@babel/types";
import * as Sentry from "@sentry/node";
import { Filesystem } from "./fs.js";
import { Context, logFailure, logWarning } from "./context.js";
import { wasmPlugin } from "./wasm.js";
import {
  ExternalPackage,
  computeExternalPackages,
  createExternalPlugin,
  findExactVersionAndDependencies,
} from "./external.js";
export { nodeFs, RecordingFs } from "./fs.js";
export type { Filesystem } from "./fs.js";

export const actionsDir = "actions";

// Returns a generator of { isDir, path, depth } for all paths
// within dirPath in some topological order (not including
// dirPath itself).
export function* walkDir(
  fs: Filesystem,
  dirPath: string,
  depth?: number,
): Generator<{ isDir: boolean; path: string; depth: number }, void, void> {
  depth = depth ?? 0;
  for (const dirEntry of fs.listDir(dirPath).sort()) {
    const childPath = path.join(dirPath, dirEntry.name);
    if (dirEntry.isDirectory()) {
      yield { isDir: true, path: childPath, depth };
      yield* walkDir(fs, childPath, depth + 1);
    } else if (dirEntry.isFile()) {
      yield { isDir: false, path: childPath, depth };
    }
  }
}

// Convex specific module environment.
type ModuleEnvironment = "node" | "isolate";

export interface Bundle {
  path: string;
  source: string;
  sourceMap?: string;
  environment: ModuleEnvironment;
}

export interface BundleHash {
  path: string;
  hash: string;
  environment: ModuleEnvironment;
}

type EsBuildResult = esbuild.BuildResult & {
  outputFiles: esbuild.OutputFile[];
  // Set of referenced external modules.
  externalModuleNames: Set<string>;
  // Set of bundled modules.
  bundledModuleNames: Set<string>;
};

async function doEsbuild(
  ctx: Context,
  dir: string,
  entryPoints: string[],
  generateSourceMaps: boolean,
  platform: esbuild.Platform,
  chunksFolder: string,
  externalPackages: Map<string, ExternalPackage>,
): Promise<EsBuildResult> {
  const external = createExternalPlugin(ctx, externalPackages);
  try {
    const result = await esbuild.build({
      entryPoints,
      bundle: true,
      platform: platform,
      format: "esm",
      target: "esnext",
      outdir: "out",
      outbase: dir,
      conditions: ["convex", "module"],
      // The wasmPlugin should be last so it doesn't run on external modules.
      plugins: [external.plugin, wasmPlugin],
      write: false,
      sourcemap: generateSourceMaps,
      splitting: true,
      chunkNames: path.join(chunksFolder, "[hash]"),
      treeShaking: true,
      minify: false,
      keepNames: true,
      metafile: true,
    });

    for (const [relPath, input] of Object.entries(result.metafile!.inputs)) {
      // TODO: esbuild outputs paths prefixed with "(disabled)"" when bundling our internal
      // udf-runtime package. The files do actually exist locally, though.
      if (
        relPath.indexOf("(disabled):") !== -1 ||
        relPath.startsWith("wasm-binary:") ||
        relPath.startsWith("wasm-stub:")
      ) {
        continue;
      }
      const absPath = path.resolve(relPath);
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
    return {
      ...result,
      externalModuleNames: external.externalModuleNames,
      bundledModuleNames: external.bundledModuleNames,
    };
  } catch (err) {
    // We don't print any error because esbuild already printed
    // all the relevant information.
    return await ctx.crash(1, "invalid filesystem data");
  }
}

export async function bundle(
  ctx: Context,
  dir: string,
  entryPoints: string[],
  generateSourceMaps: boolean,
  platform: esbuild.Platform,
  chunksFolder = "_deps",
  externalPackagesAllowList: string[] = [],
): Promise<{
  modules: Bundle[];
  externalDependencies: Map<string, string>;
  bundledModuleNames: Set<string>;
}> {
  const availableExternalPackages = await computeExternalPackages(
    ctx,
    externalPackagesAllowList,
  );
  const result = await doEsbuild(
    ctx,
    dir,
    entryPoints,
    generateSourceMaps,
    platform,
    chunksFolder,
    availableExternalPackages,
  );
  if (result.errors.length) {
    for (const error of result.errors) {
      console.log(chalk.red(`esbuild error: ${error.text}`));
    }
    return await ctx.crash(1, "invalid filesystem data");
  }
  for (const warning of result.warnings) {
    console.log(chalk.yellow(`esbuild warning: ${warning.text}`));
  }
  const sourceMaps = new Map();
  const modules: Bundle[] = [];
  const environment = platform === "node" ? "node" : "isolate";
  for (const outputFile of result.outputFiles) {
    const relPath = path.relative(path.normalize("out"), outputFile.path);
    if (path.extname(relPath) === ".map") {
      sourceMaps.set(relPath, outputFile.text);
      continue;
    }
    const posixRelPath = relPath.split(path.sep).join(path.posix.sep);
    modules.push({ path: posixRelPath, source: outputFile.text, environment });
  }
  for (const module of modules) {
    const sourceMapPath = module.path + ".map";
    const sourceMap = sourceMaps.get(sourceMapPath);
    if (sourceMap) {
      module.sourceMap = sourceMap;
    }
  }

  return {
    modules,
    externalDependencies: await externalPackageVersions(
      ctx,
      availableExternalPackages,
      result.externalModuleNames,
    ),
    bundledModuleNames: result.bundledModuleNames,
  };
}

// We could return the full list of availableExternalPackages, but this would be
// installing more packages that we need. Instead, we collect all external
// dependencies we found during bundling the /convex function, as well as their
// respective peer and optional dependencies.
async function externalPackageVersions(
  ctx: Context,
  availableExternalPackages: Map<string, ExternalPackage>,
  referencedPackages: Set<string>,
): Promise<Map<string, string>> {
  const versions = new Map<string, string>();
  const referencedPackagesQueue = Array.from(referencedPackages.keys());

  for (let i = 0; i < referencedPackagesQueue.length; i++) {
    const moduleName = referencedPackagesQueue[i];
    // This assertion is safe because referencedPackages can only contain
    // packages in availableExternalPackages.
    const modulePath = availableExternalPackages.get(moduleName)!.path;
    // Since we don't support lock files and different install commands yet, we
    // pick up the exact version installed on the local filesystem.
    const { version, peerAndOptionalDependencies } =
      await findExactVersionAndDependencies(ctx, moduleName, modulePath);
    versions.set(moduleName, version);

    for (const dependency of peerAndOptionalDependencies) {
      if (
        availableExternalPackages.has(dependency) &&
        !referencedPackages.has(dependency)
      ) {
        referencedPackagesQueue.push(dependency);
        referencedPackages.add(dependency);
      }
    }
  }

  return versions;
}

export async function bundleSchema(ctx: Context, dir: string) {
  const result = await bundle(
    ctx,
    dir,
    [path.resolve(dir, "schema.ts")],
    true,
    "browser",
  );
  return result.modules;
}

export async function bundleAuthConfig(ctx: Context, dir: string) {
  const authConfigPath = path.resolve(dir, "auth.config.js");
  const authConfigTsPath = path.resolve(dir, "auth.config.ts");
  if (ctx.fs.exists(authConfigPath) && ctx.fs.exists(authConfigTsPath)) {
    logFailure(
      ctx,
      `Found both ${authConfigPath} and ${authConfigTsPath}, choose one.`,
    );
    return await ctx.crash(1, "invalid filesystem data");
  }
  const chosenPath = ctx.fs.exists(authConfigTsPath)
    ? authConfigTsPath
    : authConfigPath;
  if (!ctx.fs.exists(chosenPath)) {
    return [];
  }
  const result = await bundle(ctx, dir, [chosenPath], true, "browser");
  return result.modules;
}

export async function doesImportConvexHttpRouter(source: string) {
  try {
    const ast = parseAST(source, {
      sourceType: "module",
      plugins: ["typescript"],
    });
    return ast.program.body.some((node) => {
      if (node.type !== "ImportDeclaration") return false;
      return node.specifiers.some((s) => {
        const specifier = s as ImportSpecifier;
        const imported = specifier.imported as Identifier;
        return imported.name === "httpRouter";
      });
    });
  } catch {
    return (
      source.match(
        /import\s*\{\s*httpRouter.*\}\s*from\s*"\s*convex\/server\s*"/,
      ) !== null
    );
  }
}

export async function entryPoints(
  ctx: Context,
  dir: string,
  verbose: boolean,
): Promise<string[]> {
  const entryPoints = [];

  const log = (line: string) => {
    if (verbose) {
      console.log(line);
    }
  };

  for (const { isDir, path: fpath, depth } of walkDir(ctx.fs, dir)) {
    if (isDir) {
      continue;
    }
    const relPath = path.relative(dir, fpath);
    const parsedPath = path.parse(fpath);
    const base = parsedPath.base;
    const extension = parsedPath.ext.toLowerCase();

    if (relPath.startsWith("_deps" + path.sep)) {
      logFailure(
        ctx,
        `The path "${fpath}" is within the "_deps" directory, which is reserved for dependencies. Please move your code to another directory.`,
      );
      return await ctx.crash(1, "invalid filesystem data");
    }

    if (depth === 0 && base.toLowerCase().startsWith("https.")) {
      const source = ctx.fs.readUtf8File(fpath);
      if (await doesImportConvexHttpRouter(source))
        logWarning(
          ctx,
          chalk.yellow(
            `Found ${fpath}. HTTP action routes will not be imported from this file. Did you mean to include http${extension}?`,
          ),
        );
      Sentry.captureMessage(
        `User code top level directory contains file ${base} which imports httpRouter.`,
        "warning",
      );
    }

    if (relPath.startsWith("_generated" + path.sep)) {
      log(chalk.yellow(`Skipping ${fpath}`));
    } else if (base.startsWith(".")) {
      log(chalk.yellow(`Skipping dotfile ${fpath}`));
    } else if (base === "README.md") {
      log(chalk.yellow(`Skipping ${fpath}`));
    } else if (base === "_generated.ts") {
      log(chalk.yellow(`Skipping ${fpath}`));
    } else if (base === "schema.ts") {
      log(chalk.yellow(`Skipping ${fpath}`));
    } else if ((base.match(/\./g) || []).length > 1) {
      log(chalk.yellow(`Skipping ${fpath} that contains multiple dots`));
    } else if (base === "tsconfig.json") {
      log(chalk.yellow(`Skipping ${fpath}`));
    } else if (relPath.endsWith(".config.js")) {
      log(chalk.yellow(`Skipping ${fpath}`));
    } else if (relPath.includes(" ")) {
      log(chalk.yellow(`Skipping ${relPath} because it contains a space`));
    } else if (base.endsWith(".d.ts")) {
      log(chalk.yellow(`Skipping ${fpath} declaration file`));
    } else if (base.endsWith(".json")) {
      log(chalk.yellow(`Skipping ${fpath} json file`));
    } else if (base.endsWith(".jsonl")) {
      log(chalk.yellow(`Skipping ${fpath} jsonl file`));
    } else {
      log(chalk.green(`Preparing ${fpath}`));
      entryPoints.push(fpath);
    }
  }

  // If using TypeScript, require that at least one line starts with `export` or `import`,
  // a TypeScript requirement. This prevents confusing type errors described in CX-5067.
  const nonEmptyEntryPoints = entryPoints.filter((fpath) => {
    // This check only makes sense for TypeScript files
    if (!fpath.endsWith(".ts") && !fpath.endsWith(".tsx")) {
      return true;
    }
    const contents = ctx.fs.readUtf8File(fpath);
    if (/^\s{0,100}(import|export)/m.test(contents)) {
      return true;
    }
    log(
      chalk.yellow(
        `Skipping ${fpath} because it has no export or import to make it a valid TypeScript module`,
      ),
    );
  });

  return nonEmptyEntryPoints;
}

// A fallback regex in case we fail to parse the AST.
export const useNodeDirectiveRegex = /^\s*("|')use node("|');?\s*$/;

function hasUseNodeDirective(
  fs: Filesystem,
  fpath: string,
  verbose: boolean,
): boolean {
  // Do a quick check for the exact string. If it doesn't exist, don't
  // bother parsing.
  const source = fs.readUtf8File(fpath);
  if (source.indexOf("use node") === -1) {
    return false;
  }

  // We parse the AST here to extract the "use node" declaration. This is more
  // robust than doing a regex. We only use regex as a fallback.
  try {
    const ast = parseAST(source, {
      // parse in strict mode and allow module declarations
      sourceType: "module",

      // esbuild supports jsx and typescript by default. Allow the same plugins
      // here too.
      plugins: ["jsx", "typescript"],
    });
    return ast.program.directives
      .map((d) => d.value.value)
      .includes("use node");
  } catch (error: any) {
    // Given that we have failed to parse, we are most likely going to fail in
    // the esbuild step, which seem to return better formatted error messages.
    // We don't throw here and fallback to regex.
    let lineMatches = false;
    for (const line of source.split("\n")) {
      if (line.match(useNodeDirectiveRegex)) {
        lineMatches = true;
        break;
      }
    }

    if (verbose) {
      // Log that we failed to parse in verbose node if we need this for debugging.
      console.warn(
        `Failed to parse ${fpath}. Use node is set to ${lineMatches} based on regex. Parse error: ${error.toString()}.`,
      );
    }

    return lineMatches;
  }
}

export function mustBeIsolate(relPath: string): boolean {
  // Check if the path without extension matches any of the static paths.
  return ["http", "crons", "schema", "auth.config"].includes(
    relPath.replace(/\.[^/.]+$/, ""),
  );
}

async function determineEnvironment(
  ctx: Context,
  dir: string,
  fpath: string,
  verbose: boolean,
): Promise<ModuleEnvironment> {
  const relPath = path.relative(dir, fpath);

  const useNodeDirectiveFound = hasUseNodeDirective(ctx.fs, fpath, verbose);
  if (useNodeDirectiveFound) {
    if (mustBeIsolate(relPath)) {
      logFailure(ctx, `"use node" directive is not allowed for ${relPath}.`);
      return await ctx.crash(1, "invalid filesystem data");
    }
    return "node";
  }

  const actionsPrefix = actionsDir + path.sep;
  if (relPath.startsWith(actionsPrefix)) {
    logFailure(
      ctx,
      `${relPath} is in /actions subfolder but has no "use node"; directive. You can now define actions in any folder and indicate they should run in node by adding "use node" directive. /actions is a deprecated way to choose Node.js environment, and we require "use node" for all files within that folder to avoid unexpected errors during the migration. See https://docs.convex.dev/functions/actions for more details`,
    );
    return await ctx.crash(1, "invalid filesystem data");
  }

  return "isolate";
}

export async function entryPointsByEnvironment(
  ctx: Context,
  dir: string,
  verbose: boolean,
) {
  const isolate = [];
  const node = [];
  for (const entryPoint of await entryPoints(ctx, dir, verbose)) {
    const environment = await determineEnvironment(
      ctx,
      dir,
      entryPoint,
      verbose,
    );
    if (environment === "node") {
      node.push(entryPoint);
    } else {
      isolate.push(entryPoint);
    }
  }

  return { isolate, node };
}
