import path from "path";
import prettier from "prettier";
import { mkdtemp, nodeFs, TempDir } from "../../bundler/fs.js";
import { entryPoints, walkDir } from "../../bundler/index.js";
import { apiCodegen } from "../codegen_templates/api.js";
import { apiCjsCodegen } from "../codegen_templates/api_cjs.js";
import { GeneratedJsWithTypes } from "../codegen_templates/common.js";
import {
  dataModel,
  dataModelWithoutSchema,
} from "../codegen_templates/dataModel.js";
import { readmeCodegen } from "../codegen_templates/readme.js";
import { serverCodegen } from "../codegen_templates/server.js";
import { tsconfigCodegen } from "../codegen_templates/tsconfig.js";
import {
  Context,
  logError,
  logMessage,
  logOutput,
} from "../../bundler/context.js";
import { typeCheckFunctionsInMode, TypeCheckMode } from "./typecheck.js";
import { readProjectConfig } from "./config.js";

/**
 * Run prettier so we don't have to think about formatting!
 *
 * This is a little sketchy because we are using the default prettier config
 * (not our user's one) but it's better than nothing.
 */
function format(source: string, filetype: string): Promise<string> {
  return prettier.format(source, { parser: filetype, pluginSearchDirs: false });
}

async function writeFile(
  ctx: Context,
  filename: string,
  source: string,
  dir: TempDir,
  dryRun: boolean,
  debug: boolean,
  quiet: boolean,
  filetype = "typescript",
) {
  const formattedSource = await format(source, filetype);
  const dest = path.join(dir.tmpPath, filename);
  if (debug) {
    logOutput(ctx, `# ${filename}`);
    logOutput(ctx, formattedSource);
    return;
  }
  if (dryRun) {
    if (ctx.fs.exists(dest)) {
      const fileText = ctx.fs.readUtf8File(dest);
      if (fileText !== formattedSource) {
        logOutput(ctx, `Command would replace file: ${dest}`);
      }
    } else {
      logOutput(ctx, `Command would create file: ${dest}`);
    }
    return;
  }

  if (!quiet) {
    logMessage(ctx, `writing ${filename}`);
  }

  nodeFs.writeUtf8File(dest, formattedSource);
}

async function writeJsWithTypes(
  ctx: Context,
  name: string,
  content: GeneratedJsWithTypes,
  codegenDir: TempDir,
  dryRun: boolean,
  debug: boolean,
  quiet: boolean,
) {
  const [jsName, dtsName] = name.endsWith(".cjs")
    ? [name, `${name.slice(0, -4)}.d.cts`]
    : name.endsWith(".mjs")
      ? [name, `${name.slice(0, -4)}.d.mts`]
      : name.endsWith(".js")
        ? [name, `${name.slice(0, -3)}.d.ts`]
        : [`${name}.js`, `${name}.d.ts`];
  await writeFile(ctx, dtsName, content.DTS, codegenDir, dryRun, debug, quiet);
  if (content.JS) {
    await writeFile(ctx, jsName, content.JS, codegenDir, dryRun, debug, quiet);
  }
}

async function doServerCodegen(
  ctx: Context,
  codegenDir: TempDir,
  dryRun: boolean,
  hasSchemaFile: boolean,
  debug: boolean,
  quiet = false,
) {
  if (hasSchemaFile) {
    await writeJsWithTypes(
      ctx,
      "dataModel",
      dataModel,
      codegenDir,
      dryRun,
      debug,
      quiet,
    );
  } else {
    await writeJsWithTypes(
      ctx,
      "dataModel",
      dataModelWithoutSchema,
      codegenDir,
      dryRun,
      debug,
      quiet,
    );
  }
  await writeJsWithTypes(
    ctx,
    "server",
    serverCodegen(),
    codegenDir,
    dryRun,
    debug,
    quiet,
  );
}

async function doApiCodegen(
  ctx: Context,
  functionsDir: string,
  codegenDir: TempDir,
  dryRun: boolean,
  debug: boolean,
  quiet = false,
  commonjs = false,
) {
  const modulePaths = (await entryPoints(ctx, functionsDir, false)).map(
    (entryPoint) => path.relative(functionsDir, entryPoint),
  );
  await writeJsWithTypes(
    ctx,
    "api",
    apiCodegen(modulePaths),
    codegenDir,
    dryRun,
    debug,
    quiet,
  );
  if (commonjs) {
    // We might generate a .d.ts file too if users need it
    // since .d.cts may not be supported in older versions of TypeScript
    await writeJsWithTypes(
      ctx,
      "api_cjs.cjs",
      apiCjsCodegen(modulePaths),
      codegenDir,
      dryRun,
      debug,
      quiet,
    );
  }
}

export async function doCodegen({
  ctx,
  functionsDirectoryPath,
  typeCheckMode,
  dryRun = false,
  debug = false,
  quiet = false,
  generateCommonJSApi = false,
}: {
  ctx: Context;
  functionsDirectoryPath: string;
  typeCheckMode: TypeCheckMode;
  dryRun?: boolean;
  debug?: boolean;
  quiet?: boolean;
  generateCommonJSApi?: boolean;
}): Promise<void> {
  const { projectConfig } = await readProjectConfig(ctx);
  // Delete the old _generated.ts because v0.1.2 used to put the react generated
  // code there
  const legacyCodegenPath = path.join(functionsDirectoryPath, "_generated.ts");
  if (ctx.fs.exists(legacyCodegenPath)) {
    if (!dryRun) {
      logError(ctx, `Deleting legacy codegen file: ${legacyCodegenPath}}`);
      ctx.fs.unlink(legacyCodegenPath);
    } else {
      logError(
        ctx,
        `Command would delete legacy codegen file: ${legacyCodegenPath}}`,
      );
    }
  }

  // Create the function dir if it doesn't already exist.
  ctx.fs.mkdir(functionsDirectoryPath, { allowExisting: true });

  const schemaPath = path.join(functionsDirectoryPath, "schema.ts");
  const hasSchemaFile = ctx.fs.exists(schemaPath);

  // Recreate the codegen directory in a temp location
  await mkdtemp("_generated", async (tempCodegenDir) => {
    // Do things in a careful order so that we always generate code in
    // dependency order.
    //
    // Ideally we would also typecheck sources before we use them. However,
    // we can't typecheck a single file while respecting the tsconfig, which can
    // produce misleading errors. Instead, we'll typecheck the generated code at
    // the end.
    //
    // The dependency chain is:
    // _generated/api.js
    // -> query and mutation functions
    // -> _generated/server.js
    // -> schema.ts
    // (where -> means "depends on")

    // 1. Use the schema.ts file to create the server codegen
    await doServerCodegen(
      ctx,
      tempCodegenDir,
      dryRun,
      hasSchemaFile,
      debug,
      quiet,
    );

    // 2. Generate API
    await doApiCodegen(
      ctx,
      functionsDirectoryPath,
      tempCodegenDir,
      dryRun,
      debug,
      quiet,
      generateCommonJSApi || projectConfig.generateCommonJSApi,
    );

    // If any files differ replace the codegen directory with its new contents
    if (!debug && !dryRun) {
      const codegenDir = path.join(functionsDirectoryPath, "_generated");
      if (!canSkipSync(ctx, tempCodegenDir, codegenDir)) {
        syncFromTemp(ctx, tempCodegenDir, codegenDir, true);
      }
    }

    // Generated code is updated, typecheck the query and mutation functions.
    await typeCheckFunctionsInMode(ctx, typeCheckMode, functionsDirectoryPath);
  });
}

function zipLongest<T>(a: T[], b: T[]): [T?, T?][] {
  return [...Array(Math.max(a.length, b.length)).keys()].map((i) => [
    a[i],
    b[i],
  ]);
}

function canSkipSync(ctx: Context, tempDir: TempDir, destDir: string) {
  if (!ctx.fs.exists(destDir)) return false;
  for (const [tmp, dest] of zipLongest(
    [...walkDir(ctx.fs, tempDir.tmpPath)],
    [...walkDir(ctx.fs, destDir)],
  )) {
    if (!tmp || !dest) return false;
    const tmpRelPath = path.relative(tempDir.tmpPath, tmp.path);
    const destRelPath = path.relative(destDir, dest.path);
    if (tmpRelPath !== destRelPath) return false;
    if (tmp.isDir !== dest.isDir) return false;
    if (tmp.isDir) continue;
    if (ctx.fs.readUtf8File(tmp.path) !== ctx.fs.readUtf8File(dest.path)) {
      return false;
    }
  }
  return true;
}

// TODO: this externalizes partial state to the watching dev server (eg vite)
// Frameworks appear to be resilient to this - but if we find issues, we
// could tighten this up per exchangedata(2) and renameat(2) - working
// under the assumption that the temp dir is on the same filesystem
// as the watched directory.
function syncFromTemp(
  ctx: Context,
  tempDir: TempDir,
  destDir: string,
  eliminateExtras: boolean, // Eliminate extra files in destDir
) {
  ctx.fs.mkdir(destDir, { allowExisting: true });
  const added = new Set();
  // Copy in the newly codegen'd files
  // Use Array.from to prevent mutation-while-iterating
  for (const { isDir, path: fpath } of Array.from(
    walkDir(ctx.fs, tempDir.tmpPath),
  )) {
    const relPath = path.relative(tempDir.tmpPath, fpath);
    const destPath = path.join(destDir, relPath);

    // Remove anything existing at the dest path.
    if (ctx.fs.exists(destPath)) {
      if (ctx.fs.stat(destPath).isDirectory()) {
        if (!isDir) {
          // converting dir -> file. Blow away old dir.
          ctx.fs.rm(destPath, { recursive: true });
        }
        // Keep directory around in this case.
      } else {
        // Blow away files
        ctx.fs.unlink(destPath);
      }
    }

    // Move in the new file
    if (isDir) {
      ctx.fs.mkdir(destPath, { allowExisting: true });
    } else {
      ctx.fs.renameFile(fpath, destPath);
    }
    added.add(destPath);
  }
  // Eliminate any extra files/dirs in the destDir. Iterate in reverse topological
  // because we're removing files.
  // Use Array.from to prevent mutation-while-iterating
  if (eliminateExtras) {
    const destEntries = Array.from(walkDir(ctx.fs, destDir)).reverse();
    for (const { isDir, path: fpath } of destEntries) {
      if (!added.has(fpath)) {
        if (isDir) {
          ctx.fs.rmdir(fpath);
        } else {
          ctx.fs.unlink(fpath);
        }
      }
    }
  }
}

export async function doInitCodegen({
  ctx,
  functionsDirectoryPath,
  dryRun = false,
  debug = false,
  quiet = false,
  overwrite = false,
}: {
  ctx: Context;
  functionsDirectoryPath: string;
  dryRun?: boolean;
  debug?: boolean;
  quiet?: boolean;
  overwrite?: boolean;
}): Promise<void> {
  await mkdtemp("convex", async (tempFunctionsDir) => {
    await doReadmeCodegen(
      ctx,
      tempFunctionsDir,
      dryRun,
      debug,
      quiet,
      overwrite ? undefined : functionsDirectoryPath,
    );
    await doTsconfigCodegen(
      ctx,
      tempFunctionsDir,
      dryRun,
      debug,
      quiet,
      overwrite ? undefined : functionsDirectoryPath,
    );
    syncFromTemp(ctx, tempFunctionsDir, functionsDirectoryPath, false);
  });
}

async function doReadmeCodegen(
  ctx: Context,
  tempFunctionsDir: TempDir,
  dryRun = false,
  debug = false,
  quiet = false,
  dontOverwriteFinalDestination?: string,
) {
  if (
    dontOverwriteFinalDestination &&
    ctx.fs.exists(path.join(dontOverwriteFinalDestination, "README.md"))
  ) {
    logMessage(ctx, `not overwriting README.md`);
    return;
  }
  await writeFile(
    ctx,
    "README.md",
    readmeCodegen(),
    tempFunctionsDir,
    dryRun,
    debug,
    quiet,
    "markdown",
  );
}

async function doTsconfigCodegen(
  ctx: Context,
  tempFunctionsDir: TempDir,
  dryRun = false,
  debug = false,
  quiet = false,
  dontOverwriteFinalDestination?: string,
) {
  if (
    dontOverwriteFinalDestination &&
    ctx.fs.exists(path.join(dontOverwriteFinalDestination, "tsconfig.json"))
  ) {
    logMessage(ctx, `not overwriting tsconfig.json`);
    return;
  }
  await writeFile(
    ctx,
    "tsconfig.json",
    tsconfigCodegen(),
    tempFunctionsDir,
    dryRun,
    debug,
    quiet,
    "json",
  );
}
