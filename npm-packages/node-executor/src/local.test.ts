import { afterEach, expect, test } from "vitest";
import { build } from "esbuild";
import { spawn, type ChildProcess } from "node:child_process";
import { mkdtemp, rm, stat } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

const POLL_INTERVAL_MS = 50;
const POLL_TIMEOUT_MS = 5_000;

async function waitFor(predicate: () => Promise<boolean>) {
  const deadline = Date.now() + POLL_TIMEOUT_MS;
  while (Date.now() < deadline) {
    if (await predicate()) {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS));
  }
  throw new Error("Timed out waiting for condition");
}

async function pathExists(filePath: string) {
  try {
    await stat(filePath);
    return true;
  } catch {
    return false;
  }
}

function processExists(pid: number) {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

let childPid: number | undefined;

afterEach(() => {
  if (childPid !== undefined && processExists(childPid)) {
    process.kill(childPid, "SIGKILL");
  }
  childPid = undefined;
});

test.skipIf(process.platform === "win32")(
  "closes its listener and exits when its supervisor is killed",
  async () => {
    const tempdir = await mkdtemp(path.join(os.tmpdir(), "node-executor-test-"));
    const bundlePath = path.join(tempdir, "local.cjs");
    const socketPath = path.join(tempdir, "executor.sock");
    let supervisor: ChildProcess | undefined;
    try {
      await build({
        bundle: true,
        entryPoints: [path.join(process.cwd(), "src", "local.ts")],
        outfile: bundlePath,
        platform: "node",
        target: "esnext",
      });
      const supervisorScript = `
      const { spawn } = require("node:child_process");
      const child = spawn(process.execPath, [
        process.argv[1],
        "--ipc-path", process.argv[2],
        "--tempdir", process.argv[3],
        "--parent-pid", String(process.pid),
      ], { stdio: ["ignore", "ignore", "ignore"] });
      process.stdout.write(String(child.pid));
      setInterval(() => {}, 1_000);
    `;
      supervisor = spawn(
        process.execPath,
        ["-e", supervisorScript, bundlePath, socketPath, tempdir],
        { stdio: ["ignore", "pipe", "inherit"] },
      );
      const output = await new Promise<string>((resolve, reject) => {
        supervisor!.stdout!.once("data", (data) => resolve(data.toString()));
        supervisor!.once("error", reject);
      });
      childPid = Number.parseInt(output, 10);
      expect(childPid).toBeGreaterThan(0);
      await waitFor(async () => pathExists(socketPath));

      supervisor.kill("SIGKILL");
      await new Promise<void>((resolve) => supervisor!.once("exit", () => resolve()));

      await waitFor(async () => !pathExists(socketPath) && !processExists(childPid!));
    } finally {
      supervisor?.kill("SIGKILL");
      await rm(tempdir, { force: true, recursive: true });
    }
  },
);
