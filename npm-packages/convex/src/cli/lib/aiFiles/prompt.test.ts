import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import fs from "fs";
import os from "os";
import path from "path";
import type { Context } from "../../../bundler/context.js";
import {
  AGENTS_MD_END_MARKER,
  AGENTS_MD_START_MARKER,
} from "../../codegen_templates/agentsmd.js";

const { mockPromptYesNo } = vi.hoisted(() => {
  return { mockPromptYesNo: vi.fn() };
});

vi.mock("@sentry/node", () => ({
  captureException: vi.fn(),
  captureMessage: vi.fn(),
}));

vi.mock("../../../bundler/log.js", () => ({
  logMessage: vi.fn(),
  logFinishedStep: vi.fn(),
}));

vi.mock("../versionApi.js", () => ({
  downloadGuidelines: vi.fn(async () => "prompt test guidelines content"),
  fetchAgentSkillsSha: vi.fn(async () => "prompt-test-sha"),
  getVersion: vi.fn(async () => ({
    kind: "ok",
    data: {
      message: null,
      guidelinesHash: "prompt-test-guidelines-hash",
      agentSkillsSha: "prompt-test-agent-skills-sha",
      disableSkillsCli: false,
    },
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

vi.mock("../utils/prompts.js", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../utils/prompts.js")>();
  return {
    ...actual,
    promptYesNo: mockPromptYesNo,
  };
});

import { attemptSetupAiFiles } from "./index.js";

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
    mockPromptYesNo.mockResolvedValue(true);
  });

  afterEach(() => {
    process.stdin.isTTY = originalIsTTY!;
    fs.rmSync(tmpDir, { recursive: true, force: true });
    vi.unstubAllEnvs();
    vi.clearAllMocks();
  });

  test("user accepts prompt: AI files are installed", async () => {
    mockPromptYesNo.mockResolvedValue(true);

    await attemptSetupAiFiles({ ctx: fakeCtx, convexDir, projectDir: tmpDir });

    expect(fs.existsSync(guidelinesPath())).toBe(true);
    expect(fs.existsSync(statePath())).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, "AGENTS.md"))).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, "CLAUDE.md"))).toBe(true);
    expect(fs.readFileSync(guidelinesPath(), "utf8")).toBe(
      "prompt test guidelines content",
    );
  });

  test("user declines prompt: no config and no AI files are written", async () => {
    mockPromptYesNo.mockResolvedValue(false);

    await attemptSetupAiFiles({ ctx: fakeCtx, convexDir, projectDir: tmpDir });

    expect(fs.existsSync(guidelinesPath())).toBe(false);
    expect(fs.existsSync(path.join(tmpDir, "AGENTS.md"))).toBe(false);
    expect(fs.existsSync(path.join(tmpDir, "CLAUDE.md"))).toBe(false);
    expect(fs.existsSync(projectConfigPath())).toBe(false);
  });

  test("non-interactive terminal skips the prompt and does not install AI files", async () => {
    process.stdin.isTTY = false;

    await attemptSetupAiFiles({ ctx: fakeCtx, convexDir, projectDir: tmpDir });

    expect(fs.existsSync(guidelinesPath())).toBe(false);
    expect(fs.existsSync(statePath())).toBe(false);
    expect(fs.existsSync(path.join(tmpDir, "AGENTS.md"))).toBe(false);
    expect(fs.existsSync(path.join(tmpDir, "CLAUDE.md"))).toBe(false);
    expect(fs.existsSync(projectConfigPath())).toBe(false);
  });

  test("existing state file updates AI files without prompting", async () => {
    const stateDir = path.join(convexDir, "_generated", "ai");
    fs.mkdirSync(stateDir, { recursive: true });
    fs.writeFileSync(
      path.join(stateDir, "ai-files.state.json"),
      JSON.stringify(
        {
          guidelinesHash: "hash",
          agentsMdSectionHash: "hash",
          claudeMdHash: "hash",
          agentSkillsSha: "sha",
          installedSkillNames: [],
        },
        null,
        2,
      ),
    );

    await attemptSetupAiFiles({ ctx: fakeCtx, convexDir, projectDir: tmpDir });

    expect(fs.existsSync(path.join(stateDir, "ai-files.state.json"))).toBe(
      true,
    );
    expect(fs.existsSync(guidelinesPath())).toBe(true);
    expect(fs.readFileSync(guidelinesPath(), "utf8")).toBe(
      "prompt test guidelines content",
    );
  });

  test("existing AGENTS.md managed section rebuilds state without prompting", async () => {
    fs.writeFileSync(
      path.join(tmpDir, "AGENTS.md"),
      [
        "# Team notes",
        "",
        AGENTS_MD_START_MARKER,
        "Managed section",
        AGENTS_MD_END_MARKER,
        "",
      ].join("\n"),
    );

    await attemptSetupAiFiles({ ctx: fakeCtx, convexDir, projectDir: tmpDir });

    expect(fs.existsSync(statePath())).toBe(true);
    expect(fs.existsSync(guidelinesPath())).toBe(true);
  });
});
