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

const LOCAL_BACKEND_INSTANCE_SECRET =
  "4361726e697461732c206c69746572616c6c79206d65616e696e6720226c6974";

export async function ensureBackendBinaryDownloaded(
  ctx: Context,
  version: { kind: "latest" } | { kind: "version"; version: string },
): Promise<{ binaryPath: string; version: string }> {
  if (version.kind === "version") {
    logVerbose(
      ctx,
      `Ensuring backend binary downloaded for version ${version.version}`,
    );
    const existingDownload = await checkForExistingDownload(
      ctx,
      version.version,
    );
    if (existingDownload !== null) {
      logVerbose(ctx, `Using existing download at ${existingDownload}`);
      return {
        binaryPath: existingDownload,
        version: version.version,
      };
    }
    const binaryPath = await downloadBinary(ctx, version.version);
    return { version: version.version, binaryPath };
  }

  logVerbose(ctx, `Ensuring latest backend binary downloaded`);
  const latest = await fetch(
    "https://github.com/get-convex/convex-backend/releases/latest",
    { redirect: "manual" },
  );
  if (latest.status !== 302) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: "Failed to get latest convex backend release",
      errForSentry: "Failed to get latest convex backend release",
    });
  }
  const latestUrl = latest.headers.get("location")!;
  const latestVersion = latestUrl.split("/").pop()!;
  logVerbose(ctx, `Latest version is ${latestVersion}`);
  return ensureBackendBinaryDownloaded(ctx, {
    kind: "version",
    version: latestVersion,
  });
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
    ctx.fs.rmdir(destDir);
    return null;
  }
  await promisify(child_process.exec)(`chmod +x ${p}`);
  return p;
}

async function downloadBinary(ctx: Context, version: string): Promise<string> {
  const downloadPath = getDownloadPath();
  if (downloadPath === null) {
    return await ctx.crash({
      exitCode: 1,
      errorType: "fatal",
      printedMessage: `Unsupported platform ${process.platform} and architecture ${process.arch} for local deployment.`,
    });
  }
  const response = await fetch(
    `https://github.com/get-convex/convex-backend/releases/download/${version}/${downloadPath}`,
  );
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
  await promisify(child_process.exec)(`chmod +x ${p}`);
  logVerbose(ctx, "Marked as executable");
  return p;
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
  cleanupHandle: () => Promise<void>;
}> {
  const { ports } = args;
  const deploymentDir = deploymentStateDir(args.deploymentName);
  ctx.fs.mkdir(deploymentDir, { recursive: true });
  const commandArgs = [
    "--port",
    ports.cloud.toString(),
    "--site-proxy-port",
    ports.site.toString(),
    "--instance-name",
    args.deploymentName,
    "--instance-secret",
    LOCAL_BACKEND_INSTANCE_SECRET,
    "--local-storage",
    path.join(deploymentDir, "convex_local_storage"),
    path.join(deploymentDir, "convex_local_backend.sqlite3"),
  ];
  const commandStr = `${args.binaryPath} ${commandArgs.join(" ")}`;
  logVerbose(ctx, `Starting local backend: \`${commandStr}\``);
  const p = child_process
    .spawn(args.binaryPath, commandArgs, { stdio: "ignore" })
    .on("exit", (code) => {
      logVerbose(
        ctx,
        `Local backend exited with code ${code}, full command \`${commandStr}\``,
      );
    });

  await ensureBackendRunning(ctx, {
    cloudPort: ports.cloud,
    deploymentName: args.deploymentName,
    maxTimeSecs: 10,
  });

  return {
    cleanupHandle: async () => {
      logVerbose(ctx, `Stopping local backend on port ${ports.cloud}`);
      p.kill("SIGTERM");
    },
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
  while (timeElapsedSecs < args.maxTimeSecs) {
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
        }
        break;
      } else {
        await new Promise((resolve) => setTimeout(resolve, 500));
        timeElapsedSecs += 0.5;
      }
    } catch (e) {
      await new Promise((resolve) => setTimeout(resolve, 500));
      timeElapsedSecs += 0.5;
    }
  }
}

export async function ensureBackendStopped(
  ctx: Context,
  args: {
    ports: {
      cloud: number;
      site?: number;
    };
    maxTimeSecs: number;
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
