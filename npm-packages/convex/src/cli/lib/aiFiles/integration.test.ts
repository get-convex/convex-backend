import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import fs from "fs";
import os from "os";
import path from "path";
import {
  checkAiFilesStaleness,
  safelyAttemptToDisableAiFiles,
  enableAiFiles,
  removeAiFiles,
  installAiFiles,
} from "./index.js";
import { statusAiFiles } from "./status.js";
import { logMessage } from "../../../bundler/log.js";
import { AGENTS_MD_START_MARKER } from "../../codegen_templates/agentsmd.js";
import { CLAUDE_MD_START_MARKER } from "../../codegen_templates/claudemd.js";

vi.mock("../../../bundler/log.js", () => ({
  logMessage: vi.fn(),
}));

vi.mock("../versionApi.js", () => ({
  downloadGuidelines: vi.fn(async () => "integration guidelines content"),
  fetchAgentSkillsSha: vi.fn(async () => "integration-sha"),
  getVersion: vi.fn(async () => ({
    kind: "ok",
    data: {
      message: null,
      guidelinesHash: "integration-guidelines-hash",
      agentSkillsSha: "integration-agent-skills-sha",
      disableSkillsCli: false,
    },
  })),
}));

vi.mock("child_process", () => ({
  default: {
    spawn: vi.fn(() => {
      return {
        stdout: { on: vi.fn() },
        stderr: { on: vi.fn() },
        on: (event: string, cb: (code: number) => void) => {
          if (event === "close") cb(0);
        },
      };
    }),
  },
}));

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

function readJson(filePath: string): any {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function makeTmpDir(): string {
  return fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
}

// ---------------------------------------------------------------------------
// Default convex/ directory (no override)
// ---------------------------------------------------------------------------

describe("ai-files integration with default convex/ directory", () => {
  let tmpDir: string;
  let convexDir: string;
  const aiDir = () => path.join(convexDir, "_generated", "ai");
  const guidelinesPath = () => path.join(aiDir(), "guidelines.md");
  const statePath = () => path.join(aiDir(), "ai-files.state.json");
  const projectConfigPath = () => path.join(tmpDir, "convex.json");

  beforeEach(() => {
    tmpDir = makeTmpDir();
    convexDir = path.join(tmpDir, "convex");
    fs.mkdirSync(convexDir, { recursive: true });
    fs.writeFileSync(path.join(convexDir, "schema.ts"), "");
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
    vi.clearAllMocks();
  });

  test("install creates guidelines, state, AGENTS.md, and CLAUDE.md", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });

    expect(fs.existsSync(guidelinesPath())).toBe(true);
    expect(fs.existsSync(statePath())).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, "AGENTS.md"))).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, "CLAUDE.md"))).toBe(true);
    expect(fs.readFileSync(guidelinesPath(), "utf8")).toBe(
      "integration guidelines content",
    );
  });

  test("preserves existing AGENTS.md content and injects managed section", async () => {
    fs.writeFileSync(
      path.join(tmpDir, "AGENTS.md"),
      "# My Project\n\nImportant team guidelines here.\n",
    );
    await installAiFiles({ projectDir: tmpDir, convexDir });
    const content = fs.readFileSync(path.join(tmpDir, "AGENTS.md"), "utf8");
    expect(content).toContain("# My Project");
    expect(content).toContain("Important team guidelines here.");
    expect(content).toContain("convex-ai-start");
    expect(content).toMatch(
      /convex[\\/]+_generated[\\/]+ai[\\/]+guidelines\.md/,
    );
  });

  test("does not overwrite pre-existing CLAUDE.md", async () => {
    fs.writeFileSync(
      path.join(tmpDir, "CLAUDE.md"),
      "My custom CLAUDE.md content\n",
    );
    await installAiFiles({ projectDir: tmpDir, convexDir });
    const content = fs.readFileSync(path.join(tmpDir, "CLAUDE.md"), "utf8");
    expect(content).toContain("My custom CLAUDE.md content");
  });

  test("second update is idempotent", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });
    const firstGuidelines = fs.readFileSync(guidelinesPath(), "utf8");
    const firstState = fs.readFileSync(statePath(), "utf8");

    await installAiFiles({ projectDir: tmpDir, convexDir });

    expect(fs.readFileSync(guidelinesPath(), "utf8")).toBe(firstGuidelines);
    expect(fs.readFileSync(statePath(), "utf8")).toBe(firstState);
  });

  test("removes legacy .cursor/rules/convex_rules.mdc", async () => {
    fs.mkdirSync(path.join(tmpDir, ".cursor", "rules"), { recursive: true });
    fs.writeFileSync(
      path.join(tmpDir, ".cursor", "rules", "convex_rules.mdc"),
      "legacy",
    );
    await installAiFiles({ projectDir: tmpDir, convexDir });
    expect(
      fs.existsSync(path.join(tmpDir, ".cursor", "rules", "convex_rules.mdc")),
    ).toBe(false);
  });

  test("staleness check nags when stored hash is stale", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });
    const state = readJson(statePath());
    state.guidelinesHash = "deliberately-stale-hash";
    fs.writeFileSync(statePath(), JSON.stringify(state, null, 2) + "\n");

    await checkAiFilesStaleness({
      canonicalGuidelinesHash: "canonical-hash",
      canonicalAgentSkillsSha: null,
      projectDir: tmpDir,
      convexDir,
    });

    expect(vi.mocked(logMessage)).toHaveBeenCalledWith(
      expect.stringContaining("out of date"),
    );
  });

  test("staleness check is silent when disabled in convex.json", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });
    await safelyAttemptToDisableAiFiles(tmpDir);
    const state = readJson(statePath());
    state.guidelinesHash = "deliberately-stale-hash";
    fs.writeFileSync(statePath(), JSON.stringify(state, null, 2) + "\n");
    vi.mocked(logMessage).mockClear();

    await checkAiFilesStaleness({
      canonicalGuidelinesHash: "canonical-hash",
      canonicalAgentSkillsSha: null,
      projectDir: tmpDir,
      convexDir,
    });

    const calls = vi.mocked(logMessage).mock.calls.map((c) => c[0]);
    expect(
      calls.find((m) => typeof m === "string" && m.includes("out of date")),
    ).toBeUndefined();
  });

  test("disable keeps files but sets convex.json preference", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });
    await safelyAttemptToDisableAiFiles(tmpDir);

    expect(readJson(projectConfigPath()).aiFiles.enabled).toBe(false);
    expect(fs.existsSync(guidelinesPath())).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, "AGENTS.md"))).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, "CLAUDE.md"))).toBe(true);
  });

  test("disable before install writes only convex.json and no AI state file", async () => {
    await safelyAttemptToDisableAiFiles(tmpDir);

    expect(readJson(projectConfigPath()).aiFiles.enabled).toBe(false);
    expect(fs.existsSync(statePath())).toBe(false);
    expect(fs.existsSync(guidelinesPath())).toBe(false);
  });

  test("remove deletes ai directory and AGENTS.md managed section", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });
    await removeAiFiles({ projectDir: tmpDir, convexDir });

    expect(fs.existsSync(aiDir())).toBe(false);
  });

  test("status reports not installed after remove", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });
    await removeAiFiles({ projectDir: tmpDir, convexDir });
    vi.mocked(logMessage).mockClear();

    await statusAiFiles({ projectDir: tmpDir, convexDir });

    expect(vi.mocked(logMessage)).toHaveBeenCalledWith(
      expect.stringContaining("not installed"),
    );
  });

  test("status reports installed and enabled after install", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });
    vi.mocked(logMessage).mockClear();

    await statusAiFiles({ projectDir: tmpDir, convexDir });

    expect(vi.mocked(logMessage)).toHaveBeenCalledWith(
      expect.stringContaining("enabled"),
    );
  });

  test("disable after CLAUDE.md user edits preserves the file", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });
    fs.appendFileSync(path.join(tmpDir, "CLAUDE.md"), "My custom note\n");
    await safelyAttemptToDisableAiFiles(tmpDir);
    expect(fs.readFileSync(path.join(tmpDir, "CLAUDE.md"), "utf8")).toContain(
      "My custom note",
    );
  });

  test("update recreates missing CLAUDE.md", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });
    fs.rmSync(path.join(tmpDir, "CLAUDE.md"), { force: true });

    await installAiFiles({ projectDir: tmpDir, convexDir });

    expect(fs.readFileSync(path.join(tmpDir, "CLAUDE.md"), "utf8")).toContain(
      "convex/_generated/ai/guidelines.md",
    );
  });

  test("enable sets enabled flag to true", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });
    await safelyAttemptToDisableAiFiles(tmpDir);
    expect(readJson(projectConfigPath()).aiFiles.enabled).toBe(false);

    await enableAiFiles({ projectDir: tmpDir, convexDir });

    expect(readJson(projectConfigPath()).aiFiles.enabled).toBe(true);
  });

  test("full cycle: disable -> remove -> enable reinstalls everything", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });
    await safelyAttemptToDisableAiFiles(tmpDir);
    await removeAiFiles({ projectDir: tmpDir, convexDir });
    expect(fs.existsSync(aiDir())).toBe(false);

    await enableAiFiles({ projectDir: tmpDir, convexDir });

    expect(fs.existsSync(guidelinesPath())).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, "AGENTS.md"))).toBe(true);
    expect(readJson(projectConfigPath()).aiFiles.enabled).toBe(true);
  });

  test("remove strips managed section from AGENTS.md but preserves user content", async () => {
    fs.writeFileSync(
      path.join(tmpDir, "AGENTS.md"),
      "# My Project\n\nTeam guidelines here.\n",
    );
    await installAiFiles({ projectDir: tmpDir, convexDir });
    const before = fs.readFileSync(path.join(tmpDir, "AGENTS.md"), "utf8");
    expect(before).toContain(AGENTS_MD_START_MARKER);

    await removeAiFiles({ projectDir: tmpDir, convexDir });

    const after = fs.readFileSync(path.join(tmpDir, "AGENTS.md"), "utf8");
    expect(after).toContain("# My Project");
    expect(after).toContain("Team guidelines here.");
    expect(after).not.toContain(AGENTS_MD_START_MARKER);
  });

  test("remove on AGENTS.md with only Convex content deletes the file", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });
    expect(fs.existsSync(path.join(tmpDir, "AGENTS.md"))).toBe(true);

    await removeAiFiles({ projectDir: tmpDir, convexDir });

    expect(fs.existsSync(path.join(tmpDir, "AGENTS.md"))).toBe(false);
  });

  test("remove deletes CLAUDE.md when empty after stripping managed section", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });
    expect(fs.existsSync(path.join(tmpDir, "CLAUDE.md"))).toBe(true);

    await removeAiFiles({ projectDir: tmpDir, convexDir });

    expect(fs.existsSync(path.join(tmpDir, "CLAUDE.md"))).toBe(false);
  });

  test("remove keeps CLAUDE.md with user content after stripping managed section", async () => {
    fs.writeFileSync(
      path.join(tmpDir, "CLAUDE.md"),
      "My project-specific Claude instructions\n",
    );
    await installAiFiles({ projectDir: tmpDir, convexDir });
    const before = fs.readFileSync(path.join(tmpDir, "CLAUDE.md"), "utf8");
    expect(before).toContain(CLAUDE_MD_START_MARKER);

    await removeAiFiles({ projectDir: tmpDir, convexDir });

    const after = fs.readFileSync(path.join(tmpDir, "CLAUDE.md"), "utf8");
    expect(after).toContain("My project-specific Claude instructions");
    expect(after).not.toContain(CLAUDE_MD_START_MARKER);
  });

  test("checkAiFilesStaleness nags when no state file exists", async () => {
    vi.mocked(logMessage).mockClear();

    await checkAiFilesStaleness({
      canonicalGuidelinesHash: null,
      canonicalAgentSkillsSha: null,
      projectDir: tmpDir,
      convexDir,
    });

    expect(vi.mocked(logMessage)).toHaveBeenCalledWith(
      expect.stringContaining("not installed"),
    );
  });

  test("checkAiFilesStaleness is silent when hashes match", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });
    const state = readJson(statePath());
    vi.mocked(logMessage).mockClear();

    await checkAiFilesStaleness({
      canonicalGuidelinesHash: state.guidelinesHash,
      canonicalAgentSkillsSha: state.agentSkillsSha,
      projectDir: tmpDir,
      convexDir,
    });

    const calls = vi.mocked(logMessage).mock.calls.map((c) => c[0]);
    expect(
      calls.find((m) => typeof m === "string" && m.includes("out of date")),
    ).toBeUndefined();
    expect(
      calls.find((m) => typeof m === "string" && m.includes("not installed")),
    ).toBeUndefined();
  });

  test("AGENTS.md managed section is replaced not duplicated on repeated updates", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });
    await installAiFiles({ projectDir: tmpDir, convexDir });
    await installAiFiles({ projectDir: tmpDir, convexDir });

    const content = fs.readFileSync(path.join(tmpDir, "AGENTS.md"), "utf8");
    const markerCount = content.split(AGENTS_MD_START_MARKER).length - 1;
    expect(markerCount).toBe(1);
  });

  test("status reports disabled state after disable", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });
    await safelyAttemptToDisableAiFiles(tmpDir);
    vi.mocked(logMessage).mockClear();

    await statusAiFiles({ projectDir: tmpDir, convexDir });

    expect(vi.mocked(logMessage)).toHaveBeenCalledWith(
      expect.stringContaining("disabled"),
    );
  });
});

// ---------------------------------------------------------------------------
// Functions directory override (convex.json.functions = "src/convex/")
// ---------------------------------------------------------------------------

describe("ai-files integration with functions directory override", () => {
  let tmpDir: string;
  let convexDir: string;
  const aiDir = () => path.join(convexDir, "_generated", "ai");
  const guidelinesPath = () => path.join(aiDir(), "guidelines.md");
  const statePath = () => path.join(aiDir(), "ai-files.state.json");
  const projectConfigPath = () => path.join(tmpDir, "convex.json");

  beforeEach(() => {
    tmpDir = makeTmpDir();
    convexDir = path.join(tmpDir, "src", "convex");
    fs.mkdirSync(convexDir, { recursive: true });
    fs.writeFileSync(path.join(convexDir, "schema.ts"), "");
    fs.writeFileSync(
      path.join(tmpDir, "convex.json"),
      JSON.stringify({ functions: "src/convex/" }, null, 2) + "\n",
      "utf8",
    );
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
    vi.clearAllMocks();
  });

  test("installs into overridden functions directory, not default convex/", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });

    expect(fs.existsSync(guidelinesPath())).toBe(true);
    expect(fs.existsSync(statePath())).toBe(true);
    expect(
      fs.existsSync(
        path.join(tmpDir, "convex", "_generated", "ai", "guidelines.md"),
      ),
    ).toBe(false);
    expect(fs.existsSync(path.join(tmpDir, "AGENTS.md"))).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, "CLAUDE.md"))).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, "src", "AGENTS.md"))).toBe(false);
  });

  test("preserves existing AGENTS.md content and injects managed section", async () => {
    fs.writeFileSync(
      path.join(tmpDir, "AGENTS.md"),
      "# Existing\n\nUser content.\n",
    );
    await installAiFiles({ projectDir: tmpDir, convexDir });
    const content = fs.readFileSync(path.join(tmpDir, "AGENTS.md"), "utf8");
    expect(content).toContain("# Existing");
    expect(content).toContain("User content.");
    expect(content).toContain("convex-ai-start");
    expect(content).toMatch(
      /src[\\/]+convex[\\/]+_generated[\\/]+ai[\\/]+guidelines\.md/,
    );
  });

  test("preserves existing CLAUDE.md content", async () => {
    fs.writeFileSync(
      path.join(tmpDir, "CLAUDE.md"),
      "My custom CLAUDE.md content\n",
      "utf8",
    );
    await installAiFiles({ projectDir: tmpDir, convexDir });
    const content = fs.readFileSync(path.join(tmpDir, "CLAUDE.md"), "utf8");
    expect(content).toContain("My custom CLAUDE.md content");
  });

  test("second update is idempotent", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });
    const firstGuidelines = fs.readFileSync(guidelinesPath(), "utf8");
    const firstState = fs.readFileSync(statePath(), "utf8");

    await installAiFiles({ projectDir: tmpDir, convexDir });

    expect(fs.readFileSync(guidelinesPath(), "utf8")).toBe(firstGuidelines);
    expect(fs.readFileSync(statePath(), "utf8")).toBe(firstState);
  });

  test("removes legacy cursor rules file during update", async () => {
    fs.mkdirSync(path.join(tmpDir, ".cursor", "rules"), { recursive: true });
    fs.writeFileSync(
      path.join(tmpDir, ".cursor", "rules", "convex_rules.mdc"),
      "legacy",
      "utf8",
    );
    await installAiFiles({ projectDir: tmpDir, convexDir });
    expect(
      fs.existsSync(path.join(tmpDir, ".cursor", "rules", "convex_rules.mdc")),
    ).toBe(false);
  });

  test("staleness check logs update nag for stale stored hash", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });
    const state = readJson(statePath());
    state.guidelinesHash = "deliberately-stale-hash";
    fs.writeFileSync(
      statePath(),
      JSON.stringify(state, null, 2) + "\n",
      "utf8",
    );

    await checkAiFilesStaleness({
      canonicalGuidelinesHash: "canonical-hash",
      canonicalAgentSkillsSha: null,
      projectDir: tmpDir,
      convexDir,
    });

    expect(vi.mocked(logMessage)).toHaveBeenCalledWith(
      expect.stringContaining("out of date"),
    );
  });

  test("disable sets convex.json preference and keeps files", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });
    await safelyAttemptToDisableAiFiles(tmpDir);

    expect(readJson(projectConfigPath()).aiFiles.enabled).toBe(false);
    expect(fs.existsSync(guidelinesPath())).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, "AGENTS.md"))).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, "CLAUDE.md"))).toBe(true);
  });

  test("remove deletes files and status reports not installed", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });
    await removeAiFiles({ projectDir: tmpDir, convexDir });
    await statusAiFiles({ projectDir: tmpDir, convexDir });

    expect(fs.existsSync(aiDir())).toBe(false);
    expect(vi.mocked(logMessage)).toHaveBeenCalledWith(
      expect.stringContaining("not installed"),
    );
  });

  test("disable after CLAUDE.md user edits preserves file", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });
    fs.appendFileSync(
      path.join(tmpDir, "CLAUDE.md"),
      "My custom note\n",
      "utf8",
    );
    await safelyAttemptToDisableAiFiles(tmpDir);
    expect(fs.readFileSync(path.join(tmpDir, "CLAUDE.md"), "utf8")).toContain(
      "My custom note",
    );
  });

  test("update recreates missing CLAUDE.md", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });
    fs.rmSync(path.join(tmpDir, "CLAUDE.md"), { force: true });

    await installAiFiles({ projectDir: tmpDir, convexDir });

    const content = fs.readFileSync(path.join(tmpDir, "CLAUDE.md"), "utf8");
    expect(content).toMatch(
      /src[\\/]+convex[\\/]+_generated[\\/]+ai[\\/]+guidelines\.md/,
    );
  });

  test("enable sets enabled flag to true and re-enables status", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });
    await safelyAttemptToDisableAiFiles(tmpDir);
    await enableAiFiles({ projectDir: tmpDir, convexDir });
    await statusAiFiles({ projectDir: tmpDir, convexDir });

    expect(readJson(projectConfigPath()).aiFiles.enabled).toBe(true);
    expect(vi.mocked(logMessage)).toHaveBeenCalledWith(
      expect.stringContaining("enabled"),
    );
  });

  test("disable + remove + enable works with overridden functions directory", async () => {
    await installAiFiles({ projectDir: tmpDir, convexDir });
    await safelyAttemptToDisableAiFiles(tmpDir);
    await removeAiFiles({ projectDir: tmpDir, convexDir });

    expect(
      fs.existsSync(path.join(tmpDir, "src", "convex", "_generated", "ai")),
    ).toBe(false);

    await enableAiFiles({ projectDir: tmpDir, convexDir });

    expect(fs.existsSync(guidelinesPath())).toBe(true);
  });
});
