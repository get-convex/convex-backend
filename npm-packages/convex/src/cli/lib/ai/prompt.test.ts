import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { PassThrough, Writable } from "stream";
import fs from "fs";
import os from "os";
import path from "path";
import type { Context } from "../../../bundler/context.js";

let testInput: PassThrough;

vi.mock("../../../bundler/log.js", () => ({
  logMessage: vi.fn(),
  logFinishedStep: vi.fn(),
}));

vi.mock("../versionApi.js", () => ({
  downloadGuidelines: vi.fn(async () => "prompt test guidelines content"),
  fetchAgentSkillsSha: vi.fn(async () => "prompt-test-sha"),
  getVersion: vi.fn(async () => ({
    message: null,
    guidelinesHash: "prompt-test-guidelines-hash",
    agentSkillsSha: "prompt-test-agent-skills-sha",
    disableSkillsCli: false,
  })),
}));

vi.mock("child_process", () => ({
  default: {
    spawn: vi.fn(() => ({
      stdout: { on: vi.fn() },
      stderr: { on: vi.fn() },
      on: (event: string, cb: (code: number) => void) => {
        if (event === "close") cb(0);
      },
    })),
  },
}));

vi.mock("@inquirer/confirm", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@inquirer/confirm")>();
  return {
    ...actual,
    default: (config: any, context: any) => {
      const output = new Writable({
        write(_c, _e, cb) {
          cb();
        },
      });
      return actual.default(config, { ...context, input: testInput, output });
    },
  };
});

import { maybeSetupAiFiles } from "./index.js";

function makeTmpDir(): string {
  return fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
}

const fakeCtx: Context = {
  fs: {
    readFile: (p: string) => fs.readFileSync(p, { encoding: "utf8" }),
    exists: (p: string) => fs.existsSync(p),
    readdir: (p: string) => fs.readdirSync(p),
    stat: (p: string) => fs.statSync(p),
    readUtf8File: (p: string) => fs.readFileSync(p, { encoding: "utf8" }),
    lstat: (p: string) => fs.lstatSync(p),
  } as any,
  deprecationMessagePrinted: false,
  crash: async (args) => {
    throw new Error(args.printedMessage ?? "crash");
  },
  registerCleanup: () => "handle",
  removeCleanup: () => null as any,
  bigBrainAuth: () => null,
  _updateBigBrainAuth: () => {},
};

describe("maybeSetupAiFiles interactive prompt", () => {
  let tmpDir: string;
  let convexDir: string;
  let originalIsTTY: boolean | undefined;

  const aiDir = () => path.join(convexDir, "_generated", "ai");
  const guidelinesPath = () => path.join(aiDir(), "guidelines.md");
  const statePath = () => path.join(aiDir(), "ai-files.state.json");
  const projectConfigPath = () => path.join(tmpDir, "convex.json");

  beforeEach(() => {
    tmpDir = makeTmpDir();
    convexDir = path.join(tmpDir, "convex");
    fs.mkdirSync(convexDir, { recursive: true });
    fs.writeFileSync(path.join(convexDir, "schema.ts"), "");
    originalIsTTY = process.stdin.isTTY;
    process.stdin.isTTY = true;
    testInput = new PassThrough();
  });

  afterEach(() => {
    process.stdin.isTTY = originalIsTTY!;
    fs.rmSync(tmpDir, { recursive: true, force: true });
    vi.unstubAllEnvs();
    vi.clearAllMocks();
  });

  test("user accepts prompt: AI files are installed", async () => {
    const result = maybeSetupAiFiles(fakeCtx, convexDir, tmpDir);

    // Simulate pressing Enter to accept the default (yes)
    testInput.emit("keypress", null, { name: "enter" });
    await result;

    expect(fs.existsSync(guidelinesPath())).toBe(true);
    expect(fs.existsSync(statePath())).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, "AGENTS.md"))).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, "CLAUDE.md"))).toBe(true);
    expect(fs.readFileSync(guidelinesPath(), "utf8")).toBe(
      "prompt test guidelines content",
    );
  });

  test("user declines prompt: no config and no AI files are written", async () => {
    const result = maybeSetupAiFiles(fakeCtx, convexDir, tmpDir);

    // Type "n" then Enter to decline
    testInput.write("n");
    testInput.emit("keypress", null, { name: "n" });
    testInput.emit("keypress", null, { name: "enter" });
    await result;

    expect(fs.existsSync(guidelinesPath())).toBe(false);
    expect(fs.existsSync(path.join(tmpDir, "AGENTS.md"))).toBe(false);
    expect(fs.existsSync(path.join(tmpDir, "CLAUDE.md"))).toBe(false);
    expect(fs.existsSync(projectConfigPath())).toBe(false);
  });

  test("agent mode skips the prompt and does not install AI files", async () => {
    vi.stubEnv("CONVEX_AGENT_MODE", "anonymous");

    await maybeSetupAiFiles(fakeCtx, convexDir, tmpDir);

    expect(fs.existsSync(guidelinesPath())).toBe(false);
    expect(fs.existsSync(statePath())).toBe(false);
    expect(fs.existsSync(path.join(tmpDir, "AGENTS.md"))).toBe(false);
    expect(fs.existsSync(path.join(tmpDir, "CLAUDE.md"))).toBe(false);
    expect(fs.existsSync(projectConfigPath())).toBe(false);
  });
});
