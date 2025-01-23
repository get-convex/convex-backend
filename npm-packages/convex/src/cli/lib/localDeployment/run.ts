import AdmZip from "adm-zip";
import { Context, logMessage, logVerbose } from "../../../bundler/context.js";
import {
  binariesDir,
  binaryZip,
  deploymentStateDir,
  executablePath,
  versionedBinaryDir,
} from "./filePaths.js";
import path from "path";
import child_process from "child_process";
import { promisify } from "util";
import { Readable } from "stream";
import { nodeFs } from "../../../bundler/fs.js";
import detect from "detect-port";
import { SENTRY_DSN } from "../utils/sentry.js";
import { createHash } from "crypto";
import { components } from "@octokit/openapi-types";
import { recursivelyDelete } from "../fsUtils.js";
import { LocalDeploymentError } from "./errors.js";
const LOCAL_BACKEND_INSTANCE_SECRET =
  "4361726e697461732c206c69746572616c6c79206d65616e696e6720226c6974";

type GitHubRelease = components["schemas"]["release"];

export async function ensureBackendBinaryDownloaded(
  ctx: Context,
  version: { kind: "latest" } | { kind: "version"; version: string },
): Promise<{ binaryPath: string; version: string }> {
  if (version.kind === "version") {
    return _ensureBackendBinaryDownloaded(ctx, version.version);
  }
  const latestVersionWithBinary = await findLatestVersionWithBinary(ctx);
  return _ensureBackendBinaryDownloaded(ctx, latestVersionWithBinary);
}

async function _ensureBackendBinaryDownloaded(
  ctx: Context,
  version: string,
): Promise<{ binaryPath: string; version: string }> {
  logVerbose(ctx, `Ensuring backend binary downloaded for version ${version}`);
  const existingDownload = await checkForExistingDownload(ctx, version);
  if (existingDownload !== null) {
    logVerbose(ctx, `Using existing download at ${existingDownload}`);
    return {
      binaryPath: existingDownload,
      version,
    };
  }
  const binaryPath = await downloadBinary(ctx, version);
  return { version, binaryPath };
}

/**
 * Finds the latest version of the convex backend that has a binary matching
 * this platform.
 */
async function findLatestVersionWithBinary(ctx: Context): Promise<string> {
  logVerbose(ctx, `Finding latest convex backend binary`);
  let releases: GitHubRelease[] = [];
  try {
    releases = (await (
      await fetch(
        "https://api.github.com/repos/get-convex/convex-backend/releases",
      )
    ).json()) as GitHubRelease[];
    if (releases.length < 1) {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: "Found no convex backend releases.",
        errForSentry: new LocalDeploymentError(
          "Found no convex backend releases.",
        ),
      });
    }
  } catch (e) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: "Failed to get latest convex backend releases",
      errForSentry: new LocalDeploymentError(e?.toString()),
    });
  }
  const targetName = getDownloadPath();
  const latest = releases[0]!;
  const latestVersion = latest.tag_name;
  logVerbose(ctx, `Latest version is ${latestVersion}`);

  let versionWithBinary: string;
  while (true) {
    const release = releases.shift()!;
    if (release.assets.find((asset) => asset.name === targetName)) {
      versionWithBinary = release.tag_name;
      break;
    }
    if (!releases.length) {
      const message = `Failed to find a convex backend release that contained ${targetName}.`;
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: message,
        errForSentry: new LocalDeploymentError(message),
      });
    }
    logVerbose(
      ctx,
      `Version ${release.tag_name} does not contain a ${targetName}, trying previous version ${releases[0].tag_name}`,
    );
  }
  return versionWithBinary;
}

/**
 *
 * @param ctx
 * @param version
 * @returns The binary path if it exists, or null
 */
async function checkForExistingDownload(
  ctx: Context,
  version: string,
): Promise<string | null> {
  const destDir = versionedBinaryDir(version);
  if (!ctx.fs.exists(destDir)) {
    return null;
  }
  const p = executablePath(version);
  if (!ctx.fs.exists(p)) {
    // This directory isn't what we expected. Remove it.
    recursivelyDelete(ctx, destDir, { force: true });
    return null;
  }
  await makeExecutable(p);
  return p;
}

async function downloadBinary(ctx: Context, version: string): Promise<string> {
  const downloadPath = getDownloadPath();
  // Note: We validate earlier that there's a binary for this platform at the specified version,
  // so in practice, we should never hit errors here.
  if (downloadPath === null) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `Unsupported platform ${process.platform} and architecture ${process.arch} for local deployment.`,
    });
  }
  const url = `https://github.com/get-convex/convex-backend/releases/download/${version}/${downloadPath}`;
  const response = await fetch(url);
  if (response.status === 404) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `Binary not found at ${url}.`,
    });
  }
  if (response.status !== 200) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `Failed to download convex backend binary at ${url}.`,
      errForSentry: new LocalDeploymentError(
        `Failed to download convex backend binary at ${url}, status ${response.status}, body ${await response.text()}`,
      ),
    });
  }
  logMessage(ctx, "Downloading convex backend");
  if (!ctx.fs.exists(binariesDir())) {
    ctx.fs.mkdir(binariesDir(), { recursive: true });
  }
  const zipLocation = binaryZip();
  if (ctx.fs.exists(zipLocation)) {
    ctx.fs.unlink(zipLocation);
  }
  const readable = Readable.fromWeb(response.body! as any);
  await nodeFs.writeFileStream(zipLocation, readable);
  logVerbose(ctx, "Downloaded zip file");

  const zip = new AdmZip(zipLocation);
  const versionDir = versionedBinaryDir(version);
  zip.extractAllTo(versionDir, true);
  logVerbose(ctx, "Extracted from zip file");
  const p = executablePath(version);
  await makeExecutable(p);
  logVerbose(ctx, "Marked as executable");
  return p;
}

async function makeExecutable(p: string) {
  switch (process.platform) {
    case "darwin":
    case "linux": {
      await promisify(child_process.exec)(`chmod +x ${p}`);
    }
  }
}

export async function runLocalBackend(
  ctx: Context,
  args: {
    ports: {
      cloud: number;
      site: number;
    };
    deploymentName: string;
    binaryPath: string;
  },
): Promise<{
  cleanupHandle: string;
}> {
  const { ports } = args;
  const deploymentDir = deploymentStateDir(args.deploymentName);
  ctx.fs.mkdir(deploymentDir, { recursive: true });
  const deploymentNameSha = createHash("sha256")
    .update(args.deploymentName)
    .digest("hex");
  const commandArgs = [
    "--port",
    ports.cloud.toString(),
    "--site-proxy-port",
    ports.site.toString(),
    "--sentry-identifier",
    deploymentNameSha,
    "--instance-name",
    args.deploymentName,
    "--instance-secret",
    LOCAL_BACKEND_INSTANCE_SECRET,
    "--local-storage",
    path.join(deploymentDir, "convex_local_storage"),
    path.join(deploymentDir, "convex_local_backend.sqlite3"),
  ];

  // Check that binary works by running with --help
  try {
    const result = child_process.spawnSync(args.binaryPath, [
      ...commandArgs,
      "--help",
    ]);
    if (result.status === 3221225781) {
      const message =
        "Local backend exited because shared libraries are missing. These may include libraries installed via 'Microsoft Visual C++ Redistributable for Visual Studio.'";
      return ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: message,
        errForSentry: new LocalDeploymentError(
          "Local backend exited with code 3221225781",
        ),
      });
    } else if (result.status !== 0) {
      const message = `Failed to run backend binary, exit code ${result.status}, error: ${result.stderr.toString()}`;
      return ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: message,
        errForSentry: new LocalDeploymentError(message),
      });
    }
  } catch (e) {
    const message = `Failed to run backend binary: ${(e as any).toString()}`;
    return ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: message,
      errForSentry: new LocalDeploymentError(message),
    });
  }
  const commandStr = `${args.binaryPath} ${commandArgs.join(" ")}`;
  logVerbose(ctx, `Starting local backend: \`${commandStr}\``);
  const p = child_process
    .spawn(args.binaryPath, commandArgs, {
      stdio: "ignore",
      env: {
        ...process.env,
        SENTRY_DSN: SENTRY_DSN,
      },
    })
    .on("exit", (code) => {
      const why = code === null ? "from signal" : `with code ${code}`;
      logVerbose(
        ctx,
        `Local backend exited ${why}, full command \`${commandStr}\``,
      );
    });
  const cleanupHandle = ctx.registerCleanup(async () => {
    logVerbose(ctx, `Stopping local backend on port ${ports.cloud}`);
    p.kill("SIGTERM");
  });

  await ensureBackendRunning(ctx, {
    cloudPort: ports.cloud,
    deploymentName: args.deploymentName,
    maxTimeSecs: 10,
  });

  return {
    cleanupHandle,
  };
}

export async function ensureBackendRunning(
  ctx: Context,
  args: {
    cloudPort: number;
    deploymentName: string;
    maxTimeSecs: number;
  },
): Promise<void> {
  logVerbose(
    ctx,
    `Ensuring backend running on port ${args.cloudPort} is running`,
  );
  const deploymentUrl = localDeploymentUrl(args.cloudPort);
  let timeElapsedSecs = 0;
  let hasShownWaiting = false;
  while (timeElapsedSecs < args.maxTimeSecs) {
    if (!hasShownWaiting && timeElapsedSecs > 2) {
      logMessage(ctx, "waiting for local backend to start...");
      hasShownWaiting = true;
    }
    try {
      const resp = await fetch(`${deploymentUrl}/instance_name`);
      if (resp.status === 200) {
        const text = await resp.text();
        if (text !== args.deploymentName) {
          return await ctx.crash({
            exitCode: 1,
            errorType: "fatal",
            printedMessage: `A different local backend ${text} is running on selected port ${args.cloudPort}`,
          });
        } else {
          // The backend is running!
          return;
        }
      } else {
        await new Promise((resolve) => setTimeout(resolve, 500));
        timeElapsedSecs += 0.5;
      }
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 500));
      timeElapsedSecs += 0.5;
    }
  }
  const message = `Local backend did not start on port ${args.cloudPort} within ${args.maxTimeSecs} seconds.`;
  return await ctx.crash({
    exitCode: 1,
    errorType: "fatal",
    printedMessage: message,
    errForSentry: new LocalDeploymentError(message),
  });
}

export async function ensureBackendStopped(
  ctx: Context,
  args: {
    ports: {
      cloud: number;
      site?: number;
    };
    maxTimeSecs: number;
    deploymentName: string;
    // Whether to allow a deployment with a different name to run on this port
    allowOtherDeployments: boolean;
  },
) {
  logVerbose(
    ctx,
    `Ensuring backend running on port ${args.ports.cloud} is stopped`,
  );
  let timeElapsedSecs = 0;
  while (timeElapsedSecs < args.maxTimeSecs) {
    const cloudPort = await detect(args.ports.cloud);
    const sitePort =
      args.ports.site === undefined ? undefined : await detect(args.ports.site);
    // Both ports are free
    if (cloudPort === args.ports.cloud && sitePort === args.ports.site) {
      return;
    }
    try {
      const instanceNameResp = await fetch(
        `${localDeploymentUrl(args.ports.cloud)}/instance_name`,
      );
      if (instanceNameResp.ok) {
        const instanceName = await instanceNameResp.text();
        if (instanceName !== args.deploymentName) {
          if (args.allowOtherDeployments) {
            return;
          }
          return await ctx.crash({
            exitCode: 1,
            errorType: "fatal",
            printedMessage: `A different local backend ${instanceName} is running on selected port ${args.ports.cloud}`,
          });
        }
      }
    } catch (error: any) {
      logVerbose(ctx, `Error checking if backend is running: ${error.message}`);
      // Backend is probably not running
      continue;
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
    timeElapsedSecs += 0.5;
  }
  return ctx.crash({
    exitCode: 1,
    errorType: "fatal",
    printedMessage: `A local backend is still running on port ${args.ports.cloud}. Please stop it and run this command again.`,
  });
}

export function localDeploymentUrl(cloudPort: number): string {
  return `http://127.0.0.1:${cloudPort}`;
}

/**
 * Get the artifact name, composed of the target convex-local-backend and
 * the Rust "target triple" appropriate for the current machine.
 **/
function getDownloadPath() {
  switch (process.platform) {
    case "darwin":
      if (process.arch === "arm64") {
        return "convex-local-backend-aarch64-apple-darwin.zip";
      } else if (process.arch === "x64") {
        return "convex-local-backend-x86_64-apple-darwin.zip";
      }
      break;
    case "linux":
      if (process.arch === "arm64") {
        return "convex-local-backend-aarch64-unknown-linux-gnu.zip";
      } else if (process.arch === "x64") {
        return "convex-local-backend-x86_64-unknown-linux-gnu.zip";
      }
      break;
    case "win32":
      return "convex-local-backend-x86_64-pc-windows-msvc.zip";
  }
  return null;
}
