import path from "path";
import prettier from "prettier";
import { withTmpDir, TempDir } from "../../bundler/fs.js";
import { entryPoints } from "../../bundler/index.js";
import { apiCodegen } from "../codegen_templates/api.js";
import { apiCjsCodegen } from "../codegen_templates/api_cjs.js";
import {
  dynamicDataModelDTS,
  noSchemaDataModelDTS,
  staticDataModelDTS,
} from "../codegen_templates/dataModel.js";
import { readmeCodegen } from "../codegen_templates/readme.js";
import { serverCodegen } from "../codegen_templates/server.js";
import { tsconfigCodegen } from "../codegen_templates/tsconfig.js";
import { Context } from "../../bundler/context.js";
import {
  logError,
  logMessage,
  logOutput,
  logVerbose,
} from "../../bundler/log.js";
import { typeCheckFunctionsInMode, TypeCheckMode } from "./typecheck.js";
import { configFilepath, readProjectConfig } from "./config.js";
import { recursivelyDelete } from "./fsUtils.js";
import {
  componentServerDTS,
  componentServerJS,
  componentServerStubDTS,
} from "../codegen_templates/component_server.js";
import { ComponentDirectory } from "./components/definition/directoryStructure.js";
import { StartPushResponse } from "./deployApi/startPush.js";
import {
  componentApiDTS,
  componentApiJs,
  componentApiStubDTS,
  rootComponentApiCJS,
} from "../codegen_templates/component_api.js";
import { functionsDir } from "./utils/utils.js";

export type CodegenOptions = {
  url?: string;
  adminKey?: string;
  dryRun: boolean;
  debug: boolean;
  typecheck: TypeCheckMode;
  init: boolean;
  commonjs: boolean;
  liveComponentSources: boolean;
  debugNodeApis: boolean;
};

export async function doCodegenForNewProject(ctx: Context) {
  const { projectConfig: existingProjectConfig } = await readProjectConfig(ctx);
  const configPath = await configFilepath(ctx);
  const functionsPath = functionsDir(configPath, existingProjectConfig);
  await doInitCodegen(ctx, functionsPath, true);
  // Disable typechecking since there isn't any code yet.
  await doCodegen(ctx, functionsPath, "disable");
}

export async function doInitCodegen(
  ctx: Context,
  functionsDir: string,
  skipIfExists: boolean,
  opts?: { dryRun?: boolean; debug?: boolean },
): Promise<void> {
  await prepareForCodegen(ctx, functionsDir, opts);
  await withTmpDir(async (tmpDir) => {
    await doReadmeCodegen(ctx, tmpDir, functionsDir, skipIfExists, opts);
    await doTsconfigCodegen(ctx, tmpDir, functionsDir, skipIfExists, opts);
  });
}

async function prepareForCodegen(
  ctx: Context,
  functionsDir: string,
  opts?: { dryRun?: boolean },
) {
  // Delete the old _generated.ts because v0.1.2 used to put the react generated
  // code there
  const legacyCodegenPath = path.join(functionsDir, "_generated.ts");
  if (ctx.fs.exists(legacyCodegenPath)) {
    if (opts?.dryRun) {
      logError(
        ctx,
        `Command would delete legacy codegen file: ${legacyCodegenPath}}`,
      );
    } else {
      logError(ctx, `Deleting legacy codegen file: ${legacyCodegenPath}}`);
      ctx.fs.unlink(legacyCodegenPath);
    }
  }

  // Create the codegen dir if it doesn't already exist.
  const codegenDir = path.join(functionsDir, "_generated");
  ctx.fs.mkdir(codegenDir, { allowExisting: true, recursive: true });
  return codegenDir;
}

export async function doCodegen(
  ctx: Context,
  functionsDir: string,
  typeCheckMode: TypeCheckMode,
  opts?: { dryRun?: boolean; generateCommonJSApi?: boolean; debug?: boolean },
) {
  const { projectConfig } = await readProjectConfig(ctx);
  const codegenDir = await prepareForCodegen(ctx, functionsDir, opts);

  await withTmpDir(async (tmpDir) => {
    // Write files in dependency order so a watching dev server doesn't
    // see inconsistent results where a file we write imports from a
    // file that doesn't exist yet. We'll collect all the paths we write
    // and then delete any remaining paths at the end.
    const writtenFiles = [];

    // First, `dataModel.d.ts` imports from the developer's `schema.js` file.
    const schemaFiles = await doDataModelCodegen(
      ctx,
      tmpDir,
      functionsDir,
      codegenDir,
      opts,
    );
    writtenFiles.push(...schemaFiles);

    // Next, the `server.d.ts` file imports from `dataModel.d.ts`.
    const serverFiles = await doServerCodegen(ctx, tmpDir, codegenDir, opts);
    writtenFiles.push(...serverFiles);

    // The `api.d.ts` file imports from the developer's modules, which then
    // import from `server.d.ts`. Note that there's a cycle here, since the
    // developer's modules could also import from the `api.{js,d.ts}` files.
    const apiFiles = await doApiCodegen(
      ctx,
      tmpDir,
      functionsDir,
      codegenDir,
      opts?.generateCommonJSApi || projectConfig.generateCommonJSApi,
      opts,
    );
    writtenFiles.push(...apiFiles);

    // Cleanup any files that weren't written in this run.
    for (const file of ctx.fs.listDir(codegenDir)) {
      if (!writtenFiles.includes(file.name)) {
        recursivelyDelete(ctx, path.join(codegenDir, file.name), opts);
      }
    }

    // Generated code is updated, typecheck the query and mutation functions.
    await typeCheckFunctionsInMode(ctx, typeCheckMode, functionsDir);
  });
}

export async function doInitialComponentCodegen(
  ctx: Context,
  tmpDir: TempDir,
  componentDirectory: ComponentDirectory,
  opts?: {
    dryRun?: boolean;
    generateCommonJSApi?: boolean;
    debug?: boolean;
    verbose?: boolean;
  },
) {
  const { projectConfig } = await readProjectConfig(ctx);

  // This component defined in a dist directory; it is probably in a node_module
  // directory, installed from a package. It is stuck with the files it has.
  // Heuristics for this:
  // - component definition has a dist/ directory as an ancestor
  // - component definition is a .js file
  // - presence of .js.map files
  // We may improve this heuristic.
  const isPublishedPackage =
    componentDirectory.definitionPath.endsWith(".js") &&
    !componentDirectory.isRoot;
  if (isPublishedPackage) {
    if (opts?.verbose) {
      logMessage(
        ctx,
        `skipping initial codegen for installed package ${componentDirectory.path}`,
      );
    }
    return;
  }

  const codegenDir = await prepareForCodegen(
    ctx,
    componentDirectory.path,
    opts,
  );

  // Write files in dependency order so a watching dev server doesn't
  // see inconsistent results where a file we write imports from a
  // file that doesn't exist yet. We'll collect all the paths we write
  // and then delete any remaining paths at the end.
  const writtenFiles = [];

  // First, `dataModel.d.ts` imports from the developer's `schema.js` file.
  const dataModelFiles = await doInitialComponentDataModelCodegen(
    ctx,
    tmpDir,
    componentDirectory,
    codegenDir,
    opts,
  );
  writtenFiles.push(...dataModelFiles);

  // Next, the `server.d.ts` file imports from `dataModel.d.ts`.
  const serverFiles = await doInitialComponentServerCodegen(
    ctx,
    componentDirectory.isRoot,
    tmpDir,
    codegenDir,
    opts,
  );
  writtenFiles.push(...serverFiles);

  // The `api.d.ts` file imports from the developer's modules, which then
  // import from `server.d.ts`. Note that there's a cycle here, since the
  // developer's modules could also import from the `api.{js,d.ts}` files.
  const apiFiles = await doInitialComponentApiCodegen(
    ctx,
    componentDirectory.isRoot,
    tmpDir,
    codegenDir,
    opts?.generateCommonJSApi || projectConfig.generateCommonJSApi,
    opts,
  );
  writtenFiles.push(...apiFiles);

  // Cleanup any files that weren't written in this run.
  for (const file of ctx.fs.listDir(codegenDir)) {
    if (!writtenFiles.includes(file.name)) {
      recursivelyDelete(ctx, path.join(codegenDir, file.name), opts);
    }
  }
}

export async function doFinalComponentCodegen(
  ctx: Context,
  tmpDir: TempDir,
  rootComponent: ComponentDirectory,
  componentDirectory: ComponentDirectory,
  startPushResponse: StartPushResponse,
  opts?: {
    dryRun?: boolean;
    debug?: boolean;
    generateCommonJSApi?: boolean;
  },
) {
  const { projectConfig } = await readProjectConfig(ctx);

  const isPublishedPackage =
    componentDirectory.definitionPath.endsWith(".js") &&
    !componentDirectory.isRoot;
  if (isPublishedPackage) {
    return;
  }

  const codegenDir = path.join(componentDirectory.path, "_generated");
  ctx.fs.mkdir(codegenDir, { allowExisting: true, recursive: true });

  // `dataModel.d.ts`, `server.d.ts` and `api.d.ts` depend on analyze results, where we
  // replace the stub generated during initial codegen with a more precise type.
  const hasSchemaFile = schemaFileExists(ctx, componentDirectory.path);
  let dataModelContents: string;
  if (hasSchemaFile) {
    if (projectConfig.codegen.staticDataModel) {
      dataModelContents = await staticDataModelDTS(
        ctx,
        startPushResponse,
        rootComponent,
        componentDirectory,
      );
    } else {
      dataModelContents = dynamicDataModelDTS();
    }
  } else {
    dataModelContents = noSchemaDataModelDTS();
  }
  const dataModelDTSPath = path.join(codegenDir, "dataModel.d.ts");
  await writeFormattedFile(
    ctx,
    tmpDir,
    dataModelContents,
    "typescript",
    dataModelDTSPath,
    opts,
  );

  const serverDTSPath = path.join(codegenDir, "server.d.ts");
  const serverContents = await componentServerDTS(componentDirectory);
  await writeFormattedFile(
    ctx,
    tmpDir,
    serverContents,
    "typescript",
    serverDTSPath,
    opts,
  );

  const apiDTSPath = path.join(codegenDir, "api.d.ts");
  const apiContents = await componentApiDTS(
    ctx,
    startPushResponse,
    rootComponent,
    componentDirectory,
    { staticApi: projectConfig.codegen.staticApi },
  );
  await writeFormattedFile(
    ctx,
    tmpDir,
    apiContents,
    "typescript",
    apiDTSPath,
    opts,
  );

  if (opts?.generateCommonJSApi || projectConfig.generateCommonJSApi) {
    const apiCjsDTSPath = path.join(codegenDir, "api_cjs.d.ts");
    await writeFormattedFile(
      ctx,
      tmpDir,
      apiContents,
      "typescript",
      apiCjsDTSPath,
      opts,
    );
  }
}

async function doReadmeCodegen(
  ctx: Context,
  tmpDir: TempDir,
  functionsDir: string,
  skipIfExists: boolean,
  opts?: { dryRun?: boolean; debug?: boolean },
) {
  const readmePath = path.join(functionsDir, "README.md");
  if (skipIfExists && ctx.fs.exists(readmePath)) {
    logVerbose(ctx, `Not overwriting README.md.`);
    return;
  }
  await writeFormattedFile(
    ctx,
    tmpDir,
    readmeCodegen(),
    "markdown",
    readmePath,
    opts,
  );
}

async function doTsconfigCodegen(
  ctx: Context,
  tmpDir: TempDir,
  functionsDir: string,
  skipIfExists: boolean,
  opts?: { dryRun?: boolean; debug?: boolean },
) {
  const tsconfigPath = path.join(functionsDir, "tsconfig.json");
  if (skipIfExists && ctx.fs.exists(tsconfigPath)) {
    logVerbose(ctx, `Not overwriting tsconfig.json.`);
    return;
  }
  await writeFormattedFile(
    ctx,
    tmpDir,
    tsconfigCodegen(),
    "json",
    tsconfigPath,
    opts,
  );
}

function schemaFileExists(ctx: Context, functionsDir: string) {
  let schemaPath = path.join(functionsDir, "schema.ts");
  let hasSchemaFile = ctx.fs.exists(schemaPath);
  if (!hasSchemaFile) {
    schemaPath = path.join(functionsDir, "schema.js");
    hasSchemaFile = ctx.fs.exists(schemaPath);
  }
  return hasSchemaFile;
}

async function doDataModelCodegen(
  ctx: Context,
  tmpDir: TempDir,
  functionsDir: string,
  codegenDir: string,
  opts?: { dryRun?: boolean; debug?: boolean },
) {
  const hasSchemaFile = schemaFileExists(ctx, functionsDir);
  const schemaContent = hasSchemaFile
    ? dynamicDataModelDTS()
    : noSchemaDataModelDTS();

  await writeFormattedFile(
    ctx,
    tmpDir,
    schemaContent,
    "typescript",
    path.join(codegenDir, "dataModel.d.ts"),
    opts,
  );
  return ["dataModel.d.ts"];
}

async function doServerCodegen(
  ctx: Context,
  tmpDir: TempDir,
  codegenDir: string,
  opts?: { dryRun?: boolean; debug?: boolean },
) {
  const serverContent = serverCodegen();
  await writeFormattedFile(
    ctx,
    tmpDir,
    serverContent.JS,
    "typescript",
    path.join(codegenDir, "server.js"),
    opts,
  );

  await writeFormattedFile(
    ctx,
    tmpDir,
    serverContent.DTS,
    "typescript",
    path.join(codegenDir, "server.d.ts"),
    opts,
  );

  return ["server.js", "server.d.ts"];
}

async function doInitialComponentServerCodegen(
  ctx: Context,
  isRoot: boolean,
  tmpDir: TempDir,
  codegenDir: string,
  opts?: { dryRun?: boolean; debug?: boolean },
) {
  await writeFormattedFile(
    ctx,
    tmpDir,
    componentServerJS(),
    "typescript",
    path.join(codegenDir, "server.js"),
    opts,
  );

  // Don't write our stub if the file already exists: It probably
  // has better type information than this stub.
  const serverDTSPath = path.join(codegenDir, "server.d.ts");
  if (!ctx.fs.exists(serverDTSPath)) {
    await writeFormattedFile(
      ctx,
      tmpDir,
      componentServerStubDTS(isRoot),
      "typescript",
      path.join(codegenDir, "server.d.ts"),
      opts,
    );
  }

  return ["server.js", "server.d.ts"];
}

async function doInitialComponentDataModelCodegen(
  ctx: Context,
  tmpDir: TempDir,
  componentDirectory: ComponentDirectory,
  codegenDir: string,
  opts?: { dryRun?: boolean; debug?: boolean },
) {
  const hasSchemaFile = schemaFileExists(ctx, componentDirectory.path);
  const dataModelContext = hasSchemaFile
    ? dynamicDataModelDTS()
    : noSchemaDataModelDTS();
  const dataModelPath = path.join(codegenDir, "dataModel.d.ts");

  // Don't write our stub if the file already exists, since it may have
  // better type information from `doFinalComponentDataModelCodegen`.
  if (!ctx.fs.exists(dataModelPath)) {
    await writeFormattedFile(
      ctx,
      tmpDir,
      dataModelContext,
      "typescript",
      dataModelPath,
      opts,
    );
  }
  return ["dataModel.d.ts"];
}

async function doInitialComponentApiCodegen(
  ctx: Context,
  isRoot: boolean,
  tmpDir: TempDir,
  codegenDir: string,
  generateCommonJSApi: boolean,
  opts?: { dryRun?: boolean; debug?: boolean },
) {
  const apiJS = componentApiJs();
  await writeFormattedFile(
    ctx,
    tmpDir,
    apiJS,
    "typescript",
    path.join(codegenDir, "api.js"),
    opts,
  );

  // Don't write the `.d.ts` stub if it already exists.
  const apiDTSPath = path.join(codegenDir, "api.d.ts");
  const apiStubDTS = componentApiStubDTS();
  if (!ctx.fs.exists(apiDTSPath)) {
    await writeFormattedFile(
      ctx,
      tmpDir,
      apiStubDTS,
      "typescript",
      apiDTSPath,
      opts,
    );
  }

  const writtenFiles = ["api.js", "api.d.ts"];

  if (generateCommonJSApi && isRoot) {
    const apiCjsJS = rootComponentApiCJS();
    await writeFormattedFile(
      ctx,
      tmpDir,
      apiCjsJS,
      "typescript",
      path.join(codegenDir, "api_cjs.cjs"),
      opts,
    );

    const cjsStubPath = path.join(codegenDir, "api_cjs.d.cts");
    if (!ctx.fs.exists(cjsStubPath)) {
      await writeFormattedFile(
        ctx,
        tmpDir,
        apiStubDTS,
        "typescript",
        cjsStubPath,
        opts,
      );
    }
    writtenFiles.push("api_cjs.cjs", "api_cjs.d.cts");
  }

  return writtenFiles;
}

async function doApiCodegen(
  ctx: Context,
  tmpDir: TempDir,
  functionsDir: string,
  codegenDir: string,
  generateCommonJSApi: boolean,
  opts?: { dryRun?: boolean; debug?: boolean },
) {
  const absModulePaths = await entryPoints(ctx, functionsDir);
  const modulePaths = absModulePaths.map((p) => path.relative(functionsDir, p));

  const apiContent = apiCodegen(modulePaths);
  await writeFormattedFile(
    ctx,
    tmpDir,
    apiContent.JS,
    "typescript",
    path.join(codegenDir, "api.js"),
    opts,
  );
  await writeFormattedFile(
    ctx,
    tmpDir,
    apiContent.DTS,
    "typescript",
    path.join(codegenDir, "api.d.ts"),
    opts,
  );
  const writtenFiles = ["api.js", "api.d.ts"];

  if (generateCommonJSApi) {
    const apiCjsContent = apiCjsCodegen(modulePaths);
    await writeFormattedFile(
      ctx,
      tmpDir,
      apiCjsContent.JS,
      "typescript",
      path.join(codegenDir, "api_cjs.cjs"),
      opts,
    );
    await writeFormattedFile(
      ctx,
      tmpDir,
      apiCjsContent.DTS,
      "typescript",
      path.join(codegenDir, "api_cjs.d.cts"),
      opts,
    );
    writtenFiles.push("api_cjs.cjs", "api_cjs.d.cts");
  }

  return writtenFiles;
}

async function writeFormattedFile(
  ctx: Context,
  tmpDir: TempDir,
  contents: string,
  filetype: string,
  destination: string,
  options?: {
    dryRun?: boolean;
    debug?: boolean;
  },
) {
  // Run prettier so we don't have to think about formatting!
  //
  // This is a little sketchy because we are using the default prettier config
  // (not our user's one) but it's better than nothing.
  const formattedContents = await prettier.format(contents, {
    parser: filetype,
    pluginSearchDirs: false,
  });
  if (options?.debug) {
    // NB: The `test_codegen_projects_are_up_to_date` smoke test depends
    // on this output format.
    logOutput(ctx, `# ${path.resolve(destination)}`);
    logOutput(ctx, formattedContents);
    return;
  }
  try {
    const existing = ctx.fs.readUtf8File(destination);
    if (existing === formattedContents) {
      return;
    }
  } catch (err: any) {
    if (err.code !== "ENOENT") {
      // eslint-disable-next-line no-restricted-syntax
      throw err;
    }
  }
  if (options?.dryRun) {
    logOutput(ctx, `Command would write file: ${destination}`);
    return;
  }
  const tmpPath = tmpDir.writeUtf8File(formattedContents);
  ctx.fs.swapTmpFile(tmpPath, destination);
}
