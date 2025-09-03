import { CanonicalizedModulePath } from "./convex";
import * as fs from "node:fs";
import * as stream from "node:stream";
import os from "node:os";
import path from "node:path";
import AdmZip from "adm-zip";
import concat from "concat-stream";

import fetch from "node-fetch";
import { createHash } from "node:crypto";
import { logDebug, logDurationMs } from "./log";
import { performance } from "node:perf_hooks";
import { fileURLToPath } from "node:url";

export type SourcePackage = {
  // Deprecated fields
  uri: string;
  key: string;
  sha256: string;

  bundled_source: Package;
  external_deps?: Package | null;
};

export type Package = {
  uri: string;
  key: string;
  sha256: string;
};

type ModuleEnvironment = "node" | "isolate";

type MetadataJson = {
  modulePaths: string[];
  moduleEnvironments: Map<string, ModuleEnvironment>;
  externalDepsStorageKey?: string;
};

async function download(
  uri: string,
): Promise<fs.ReadStream | NodeJS.ReadableStream> {
  const url = new URL(uri);
  if (url.protocol === "file:") {
    return fs.createReadStream(fileURLToPath(uri));
  } else {
    const response = await fetch(uri);
    if (!response.ok) {
      throw new Error(`Failed to fetch ${uri}: ${response.statusText}`);
    }
    return response.body;
  }
}

function parseMetadataFile(contents: string): MetadataJson {
  const metadataJson = JSON.parse(contents.toString()) as {
    modulePaths: string[];
    moduleEnvironments: [string, ModuleEnvironment][] | undefined;
    externalDepsStorageKey?: string;
  };

  metadataJson.modulePaths.sort();

  // Old versions didn't populate moduleEnvironments.
  if (metadataJson.moduleEnvironments === undefined) {
    metadataJson.moduleEnvironments = [];
    for (const path of metadataJson.modulePaths) {
      const environment = path.startsWith("actions/") ? "node" : "isolate";
      metadataJson.moduleEnvironments.push([path, environment]);
    }
  }

  const moduleEnvironmentsMap = new Map();
  for (const [path, environment] of metadataJson.moduleEnvironments) {
    moduleEnvironmentsMap.set(path, environment);
  }

  return {
    modulePaths: metadataJson.modulePaths,
    moduleEnvironments: moduleEnvironmentsMap,
    externalDepsStorageKey: metadataJson.externalDepsStorageKey,
  };
}

/// Downloads source package and external deps package, if necessary,
/// populating cache with result. Links external deps package into
/// local source package directory.
export async function maybeDownloadAndLinkPackages(
  sourcePackage: SourcePackage,
): Promise<LocalSourcePackage> {
  // If we've previously downloaded and cached this source package, we've already linked the necessary
  // external modules and so there is no more work left to do, so return.
  const local = availableSourcePackages.get(sourcePackage.key);
  if (local !== undefined) {
    return local;
  }

  const sourcePackagePromise = downloadSourcePackage(
    sourcePackage.bundled_source,
  );
  const externalPackagePromise = sourcePackage.external_deps
    ? maybeDownloadExternalPackage(sourcePackage.external_deps)
    : null;
  const [localPackage, externalPackage] = await Promise.all([
    sourcePackagePromise,
    externalPackagePromise,
  ]);

  // Do symlinking of external package into local source package node_modules folder.
  //
  // This symlink is necessary only if there does not already exist a node_modules folder
  // in the source (localPackage) directory. Why? If a package already exists in the localPackage.dir
  // directory, we can be sure it is up-to-date since a given source package can only ever map to
  // one set of external deps. If external deps change, a new source package is created.
  //
  // If we reach this point and a valid externalPackage exists for this source, we can unconditionally
  // symlink since we can be sure that the local package was not previously downloaded and cached, otherwise
  // this function would have returned earlier. Thus, the local package directory has been freshly downloaded
  // and so no node_modules folder can exist already.
  if (externalPackage) {
    logDebug(
      `Attempting symlink from ${externalPackage.dir}/node_modules to ${localPackage.dir}/node_modules`,
    );
    await fs.promises.symlink(
      `${externalPackage.dir}/node_modules`,
      `${localPackage.dir}/node_modules`,
      "dir",
    );
  }

  // Save result for next time
  availableSourcePackages.set(sourcePackage.key, localPackage);

  return localPackage;
}

// Downloads sourcePackage and unzips it into `source/${sourcePackage.key}/modules`
async function downloadSourcePackage(
  sourcePackage: Package,
): Promise<LocalSourcePackage> {
  const start = performance.now();
  logDebug("Downloading source package...");

  // First cleanup any previously downloaded packages in order to not run
  // out of space.
  await cleanupSourcePackages();
  logDurationMs("cleanupTime", start);

  // Create directory and do download in parallel
  const dir = path.join(os.tmpdir(), `source/${sourcePackage.key}`);
  const dirPromise = createFreshDir(dir);
  const downloadPackagePromise = download(sourcePackage.uri);
  const [sourcePackageStream, ..._] = await Promise.all([
    downloadPackagePromise,
    dirPromise,
  ]);

  // Process local source package
  const result = await processSourcePackageStream(
    dir,
    sourcePackage,
    sourcePackageStream,
  );
  logDurationMs("sourceDownloadTime", start);

  return result;
}

// Downloads externalPackage and unzips it into `externals/${externalPackage.key}/node_modules`.
async function maybeDownloadExternalPackage(
  externalPackage: Package,
): Promise<ExternalDepsPackage> {
  const start = performance.now();
  const externalDeps =
    availableExternalPackages.get(externalPackage.key) || null;

  if (!externalDeps) {
    logDebug("External Package not available locally");

    // Cleanup other external dependency packages to not run out of disk space
    await cleanupExternalPackages();
    logDurationMs("cleanupExternalPackages", start);

    // Create directory and do download in parallel
    const downloadStart = performance.now();
    const dir = path.join(os.tmpdir(), `external_deps/${externalPackage.key}`);
    const dirPromise = createFreshDir(dir);
    const downloadPackagePromise = download(externalPackage.uri);
    const [externalPackageStream, ..._] = await Promise.all([
      downloadPackagePromise,
      dirPromise,
    ]);
    logDurationMs("downloadExternalsTime", downloadStart);

    // Process the external package download readable stream by checking hash and writing to dir
    await processExternalPackageStream(
      dir,
      externalPackage,
      externalPackageStream,
    );
    const result: ExternalDepsPackage = { dir, dynamicallyDownloaded: true };

    // Save result for next time
    availableExternalPackages.set(externalPackage.key, result);

    logDurationMs("externalDepsProcessingTime", start);
    return result;
  } else {
    logDebug("External Package available locally");
    return externalDeps;
  }
}

async function createFreshDir(dir: string) {
  await fs.promises.rm(dir, { recursive: true, force: true });
  // fs.promises.mkdir sometimes fails with ENOENT (the real error might be ENOSPC, but we're not sure)
  fs.mkdirSync(dir, { recursive: true, mode: 0o744 });
}

async function streamToBuffer(
  readableStream: fs.ReadStream | NodeJS.ReadableStream,
): Promise<Buffer> {
  return new Promise((resolve, reject) => {
    // Use concat-stream to collect the stream data into a single buffer
    const concatStream = concat((data) => {
      resolve(data);
    });

    // Handle the stream with pipeline
    stream.promises.pipeline(readableStream, concatStream).catch(reject);
  });
}

async function processPackageStream(
  sha256Digest: string,
  packageStream: fs.ReadStream | NodeJS.ReadableStream,
): Promise<Buffer> {
  // Create hashing stream
  const hash = createHash("sha256");
  packageStream.on("data", (chunk) => hash.update(chunk));
  const hashDone = new Promise((resolve, reject) => {
    packageStream
      .on("end", () => {
        resolve(null);
      })
      .on("error", (err) => {
        reject(err);
      });
  });

  const bufWriteDone = streamToBuffer(packageStream);

  // Make sure that all promises have been populated and the hash is done calculating
  const [buf, _] = await Promise.all([bufWriteDone, hashDone]);

  // Assert checksum matches
  const digest = hash.digest().toString("base64url");
  if (digest !== sha256Digest) {
    throw new Error(
      `Invalid checksum, got ${digest}, expected ${sha256Digest}`,
    );
  }

  return buf;
}

async function unzipFile(
  zipBuffer: Buffer,
  outputDir: string,
  entryValidator: ((entry: AdmZip.IZipEntry) => void) | null,
) {
  const zip = new AdmZip(zipBuffer);
  zip.extractAllTo(outputDir, true);

  const results = [];
  for (const zipEntry of zip.getEntries()) {
    if (entryValidator) entryValidator(zipEntry);
    results.push(zipEntry.entryName);
  }
  return results;
}

async function processExternalPackageStream(
  dir: string,
  externalPackage: Package,
  externalStream: fs.ReadStream | NodeJS.ReadableStream,
): Promise<void> {
  const entryValidator = (entry: AdmZip.IZipEntry) => {
    if (!entry.entryName.startsWith("node_modules/")) {
      throw new Error(`found incorrect package entry ${entry.entryName}`);
    }
  };

  const startUnzip = performance.now();
  const zipBuffer = await processPackageStream(
    externalPackage.sha256,
    externalStream,
  );
  logDurationMs("unzipExternalsTime", startUnzip);

  const startWrites = performance.now();
  await unzipFile(zipBuffer, dir, entryValidator);
  logDurationMs("externalsWritesTime", startWrites);
}

type LocalSourcePackage = {
  dir: string;
  /**
   * The modules included in the package and that could contain Convex functions.
   * This doesn’t include bundler chunks (files in /_deps/).
   */
  modules: Set<CanonicalizedModulePath>;
  dynamicallyDownloaded: boolean;
};

type ExternalDepsPackage = {
  dir: string;
  dynamicallyDownloaded: boolean;
};

async function processSourcePackageStream(
  dir: string,
  sourcePackage: Package,
  sourceStream: fs.ReadStream | NodeJS.ReadableStream,
): Promise<LocalSourcePackage> {
  const startUnzip = performance.now();
  const zipBuffer = await processPackageStream(
    sourcePackage.sha256,
    sourceStream,
  );
  logDurationMs("unzipSourceTime", startUnzip);

  // After finishing the pipeline, await on each File's buffer.
  const startWrites = performance.now();
  const entries = await unzipFile(zipBuffer, dir, null);
  const actualModulePaths = entries
    .filter(
      (entry) =>
        entry !== "metadata.json" &&
        // Some ZIP implementations store entries for directories themselves
        // (https://unix.stackexchange.com/a/743512/485280)
        // The Rust implementation we use in production doesn’t do it, but some
        // implementations (including the `archiver` npm package used in
        // node-executor integration tests) do so, so we are filtering them out.
        !entry.endsWith("/"),
    )
    .map((entry) => entry.substring("modules/".length));
  await fs.promises.chmod(`${dir}/metadata.json`, "444");
  const metadataJson = parseMetadataFile(
    await fs.promises.readFile(`${dir}/metadata.json`, {
      encoding: "utf-8",
    }),
  );

  // Old packages don't have project.json
  createPackageJsonIfMissing(dir);
  logDurationMs("sourceWritesTime", startWrites);
  actualModulePaths.sort();
  if (
    JSON.stringify(metadataJson.modulePaths) !==
    JSON.stringify(actualModulePaths)
  ) {
    throw new Error(
      `metadata.json incorrect. ${JSON.stringify(
        metadataJson.modulePaths,
      )}\n !=\n ${JSON.stringify(actualModulePaths)}`,
    );
  }

  const modules = modulesFromMetadataJson(metadataJson);
  return {
    dir,
    modules,
    dynamicallyDownloaded: true,
  };
}

export const availableSourcePackages = new Map<string, LocalSourcePackage>();
export const availableExternalPackages = new Map<string, ExternalDepsPackage>();

/**
 * Prepopulates source and external deps caches if this Lambda was pushed with source and, optionally,
 * an external deps package.
 *
 * This source is pushed in ${__dirname}/source/ and includes the following folders:
 * - modules/ storing all user code
 * - [optional] node_modules/ storing external dependencies
 * - metadata.json storing MetadataJson object
 * - package.json
 */
export async function populatePrebuildPackages() {
  const sourceDir = path.join(__dirname, "/source");
  if (fs.statSync(sourceDir, { throwIfNoEntry: false }) === undefined) {
    // If we weren't pushed with source, skip prepopulations
    return;
  }

  const pkgs = fs.readdirSync(sourceDir);
  for (const pkg of pkgs) {
    const pkgDir = path.join(sourceDir, pkg);
    let metadata: MetadataJson;
    let modules: Set<string>;
    try {
      metadata = parseMetadataFile(
        fs.readFileSync(`${pkgDir}/metadata.json`, { encoding: "utf-8" }),
      );
      modules = modulesFromMetadataJson(metadata);
    } catch (e: any) {
      logDebug(`Failed to parse metadata.json during prebuild, skipping: ${e}`);
      continue;
    }
    availableSourcePackages.set(pkg, {
      dir: pkgDir,
      modules,
      dynamicallyDownloaded: false,
    });

    if (
      metadata.externalDepsStorageKey &&
      fs.existsSync(path.join(pkgDir, "node_modules"))
    ) {
      logDebug(
        `Prepopulating external deps with storage key ${metadata.externalDepsStorageKey} in ${pkgDir}`,
      );
      availableExternalPackages.set(metadata.externalDepsStorageKey, {
        dir: pkgDir,
        dynamicallyDownloaded: false,
      });
    }
  }
}

export function createPackageJsonIfMissing(dir: string) {
  // Ensure package.json exists. This is required so node knowns to execute
  // the user modules as ESM, since they have .js and not .mjs extension.
  const packageJsonPath = path.join(dir, "package.json");
  if (fs.existsSync(packageJsonPath)) {
    return;
  }
  fs.writeFileSync(packageJsonPath, `{ "type": "module" }`);
}

function modulesFromMetadataJson(
  metadataJson: MetadataJson,
): Set<CanonicalizedModulePath> {
  const modules: Set<string> = new Set();
  for (const path of metadataJson.modulePaths) {
    if (path.startsWith("_deps/")) {
      // Ignore bundler chunks since they don’t contain Convex function definitions.
      continue;
    } else if (path.endsWith(".js")) {
      // Only load node files.
      const environment = metadataJson.moduleEnvironments.get(path);
      if (!environment) {
        throw new Error(`Missing environment for ${path}`);
      }
      if (environment !== "node") {
        continue;
      }
      modules.add(path);
    } else if (path.endsWith(".js.map")) {
      continue;
    } else {
      throw new Error(`Invalid path in archive: ${path}`);
    }
  }
  return modules;
}

// Delete all dynamically downloaded source packages.
export async function cleanupSourcePackages() {
  for (const [key, local] of availableSourcePackages) {
    if (local.dynamicallyDownloaded) {
      availableSourcePackages.delete(key);
      await fs.promises.rm(local.dir, { recursive: true, force: true });
    }
  }
}

// Delete all external dependency packages. This doesn't distinguish between
// prebuilt and non-prebuilt packages, like cleanupSourcePackages, since there
// is no prebuild of external packages yet.
export async function cleanupExternalPackages() {
  for (const [key, pkg] of availableExternalPackages) {
    if (pkg.dynamicallyDownloaded) {
      availableExternalPackages.delete(key);
      await fs.promises.rm(pkg.dir, { recursive: true, force: true });
    }
  }
}
