import { describe, test, expect, vi, beforeEach, afterEach } from "vitest";
import * as Sentry from "@sentry/node";
import { logMessage } from "../../../bundler/log.js";
import { readAiConfig, writeAiConfig } from "./config.js";
import {
  downloadGuidelines,
  fetchAgentSkillsSha,
  getVersion,
} from "../versionApi.js";
import fs from "fs";
import os from "os";
import path from "path";
import {
  injectAgentsMdSection,
  injectClaudeMdSection,
  checkAiFilesStaleness,
  updateAiFiles,
  removeAiFiles,
  disableAiFiles,
  writeDisabledAiConfig,
  statusAiFiles,
} from "./index.js";
import {
  AGENTS_MD_START_MARKER,
  AGENTS_MD_END_MARKER,
} from "../../codegen_templates/agentsmd.js";
import {
  CLAUDE_MD_START_MARKER,
  CLAUDE_MD_END_MARKER,
} from "../../codegen_templates/claudemd.js";

// ---------------------------------------------------------------------------
// injectAgentsMdSection — tested with real temp directories to exercise the
// actual file I/O and string-surgery logic without complex mock wiring.
// ---------------------------------------------------------------------------

describe("injectAgentsMdSection", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  const section = `${AGENTS_MD_START_MARKER}\n## Convex\nRead guidelines.\n${AGENTS_MD_END_MARKER}`;

  test("creates AGENTS.md when it does not exist", async () => {
    await injectAgentsMdSection(section, tmpDir);

    const content = fs.readFileSync(path.join(tmpDir, "AGENTS.md"), "utf8");
    expect(content).toContain(AGENTS_MD_START_MARKER);
    expect(content).toContain(AGENTS_MD_END_MARKER);
    expect(content).toContain("## Convex");
  });

  test("appends to an existing AGENTS.md that has no Convex section", async () => {
    const existing = "# My project\n\nSome existing content.\n";
    fs.writeFileSync(path.join(tmpDir, "AGENTS.md"), existing);

    await injectAgentsMdSection(section, tmpDir);

    const content = fs.readFileSync(path.join(tmpDir, "AGENTS.md"), "utf8");
    expect(content).toContain("# My project");
    expect(content).toContain("Some existing content.");
    expect(content).toContain(AGENTS_MD_START_MARKER);
    expect(content).toContain("## Convex");
  });

  test("replaces an existing Convex section when markers are present", async () => {
    const oldSection = `${AGENTS_MD_START_MARKER}\n## Convex\nOld content.\n${AGENTS_MD_END_MARKER}`;
    const existing = `# My project\n\n${oldSection}\n`;
    fs.writeFileSync(path.join(tmpDir, "AGENTS.md"), existing);

    const newSection = `${AGENTS_MD_START_MARKER}\n## Convex\nNew content.\n${AGENTS_MD_END_MARKER}`;
    await injectAgentsMdSection(newSection, tmpDir);

    const content = fs.readFileSync(path.join(tmpDir, "AGENTS.md"), "utf8");
    expect(content).toContain("New content.");
    expect(content).not.toContain("Old content.");
    // Only one occurrence of the start marker
    expect(content.split(AGENTS_MD_START_MARKER).length - 1).toBe(1);
  });

  test("preserves content before and after an existing Convex section", async () => {
    const oldSection = `${AGENTS_MD_START_MARKER}\n## Convex\nOld.\n${AGENTS_MD_END_MARKER}`;
    const existing = `# Before\n\n${oldSection}\n\n# After\n`;
    fs.writeFileSync(path.join(tmpDir, "AGENTS.md"), existing);

    await injectAgentsMdSection(section, tmpDir);

    const content = fs.readFileSync(path.join(tmpDir, "AGENTS.md"), "utf8");
    expect(content).toContain("# Before");
    expect(content).toContain("# After");
  });

  test("returns a non-null hash of the written content", async () => {
    const hash = await injectAgentsMdSection(section, tmpDir);
    expect(typeof hash).toBe("string");
    expect(hash!.length).toBeGreaterThan(0);
  });

  test("returns hash of the section content, not the entire file", async () => {
    fs.writeFileSync(
      path.join(tmpDir, "AGENTS.md"),
      "# My project\n\nExisting content.\n",
    );

    const hash = await injectAgentsMdSection(section, tmpDir);

    const { hashSha256 } = await import("../utils/hash.js");
    expect(hash).toBe(hashSha256(section));
  });
});

describe("injectClaudeMdSection", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  const section = `${CLAUDE_MD_START_MARKER}\n## Convex\nRead guidelines.\n${CLAUDE_MD_END_MARKER}`;

  test("creates CLAUDE.md when it does not exist", async () => {
    const result = await injectClaudeMdSection(section, tmpDir);

    const content = fs.readFileSync(path.join(tmpDir, "CLAUDE.md"), "utf8");
    expect(content).toContain(CLAUDE_MD_START_MARKER);
    expect(content).toContain(CLAUDE_MD_END_MARKER);
    expect(result.didWrite).toBe(true);
  });

  test("appends managed section to existing CLAUDE.md content", async () => {
    fs.writeFileSync(
      path.join(tmpDir, "CLAUDE.md"),
      "My custom instructions\n",
    );

    const result = await injectClaudeMdSection(section, tmpDir);

    const content = fs.readFileSync(path.join(tmpDir, "CLAUDE.md"), "utf8");
    expect(content).toContain("My custom instructions");
    expect(content).toContain(CLAUDE_MD_START_MARKER);
    expect(result.didWrite).toBe(true);
  });

  test("replaces managed section without touching user content", async () => {
    const oldSection = `${CLAUDE_MD_START_MARKER}\nOld\n${CLAUDE_MD_END_MARKER}`;
    fs.writeFileSync(
      path.join(tmpDir, "CLAUDE.md"),
      `# Header\n\n${oldSection}\n\n# Footer\n`,
      "utf8",
    );

    await injectClaudeMdSection(section, tmpDir);

    const content = fs.readFileSync(path.join(tmpDir, "CLAUDE.md"), "utf8");
    expect(content).toContain("# Header");
    expect(content).toContain("# Footer");
    expect(content).toContain("## Convex");
    expect(content).not.toContain("Old");
  });
});

// ---------------------------------------------------------------------------
// checkAiFilesStaleness — mock-based: logic only, no real I/O needed.
// ---------------------------------------------------------------------------

vi.mock("@sentry/node", () => ({
  captureException: vi.fn(),
  captureMessage: vi.fn(),
}));

vi.mock("../../../bundler/log.js", () => ({
  logMessage: vi.fn(),
}));

vi.mock("./config.js", () => ({
  readAiConfig: vi.fn(),
  writeAiConfig: vi.fn(),
}));

vi.mock("../versionApi.js", () => ({
  downloadGuidelines: vi.fn(),
  fetchAgentSkillsSha: vi.fn(),
  getVersion: vi.fn(),
}));

vi.mock("child_process", () => ({
  default: {
    spawn: vi.fn(() => {
      const emitter = { on: vi.fn() };
      // Immediately simulate a successful exit.
      emitter.on.mockImplementation(
        (event: string, cb: (arg: number) => void) => {
          if (event === "close") cb(0);
        },
      );
      return emitter;
    }),
  },
}));

const mockLogMessage = vi.mocked(logMessage);
const mockReadAiConfig = vi.mocked(readAiConfig);
const mockWriteAiConfig = vi.mocked(writeAiConfig);
const mockDownloadGuidelines = vi.mocked(downloadGuidelines);
const mockFetchAgentSkillsSha = vi.mocked(fetchAgentSkillsSha);
const mockGetVersion = vi.mocked(getVersion);
const mockCaptureException = vi.mocked(Sentry.captureException);

/** Minimal valid config used across tests; includes all required fields. */
const baseConfig = {
  guidelinesHash: null,
  agentsMdSectionHash: null,
  claudeMdHash: null,
  agentSkillsSha: null,
  installedSkillNames: [] as string[],
  disableStalenessMessage: false,
};

describe("checkAiFilesStaleness", () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(() => vi.resetAllMocks());

  const dummyProjectDir = "/tmp/test-project";
  const dummyConvexDir = "/tmp/test-project/convex";

  test("logs install nudge when no config file exists, even with null canonical values", async () => {
    mockReadAiConfig.mockResolvedValue(null);

    await checkAiFilesStaleness(null, null, dummyProjectDir, dummyConvexDir);

    expect(mockReadAiConfig).toHaveBeenCalled();
    expect(mockLogMessage).toHaveBeenCalledWith(
      expect.stringContaining("npx convex ai-files install"),
    );
    expect(mockLogMessage).toHaveBeenCalledWith(
      expect.stringContaining("not installed"),
    );
  });

  test("does nothing when both canonical values are null but config exists (version server unavailable)", async () => {
    mockReadAiConfig.mockResolvedValue({
      ...baseConfig,
      guidelinesHash: "some-hash",
    });

    await checkAiFilesStaleness(null, null, dummyProjectDir, dummyConvexDir);

    expect(mockLogMessage).not.toHaveBeenCalled();
  });

  test("logs install nudge when no config file exists (never set up)", async () => {
    mockReadAiConfig.mockResolvedValue(null);

    await checkAiFilesStaleness(
      "canonical-hash",
      null,
      dummyProjectDir,
      dummyConvexDir,
    );

    expect(mockLogMessage).toHaveBeenCalledWith(
      expect.stringContaining("npx convex ai-files install"),
    );
    expect(mockLogMessage).toHaveBeenCalledWith(
      expect.stringContaining("npx convex ai-files disable"),
    );
    expect(mockLogMessage).toHaveBeenCalledWith(
      expect.stringContaining("not installed"),
    );
  });

  test("does nothing when config has disableStalenessMessage=true (user opted out)", async () => {
    mockReadAiConfig.mockResolvedValue({
      ...baseConfig,
      disableStalenessMessage: true,
    });

    await checkAiFilesStaleness(
      "canonical-hash",
      null,
      dummyProjectDir,
      dummyConvexDir,
    );

    expect(mockLogMessage).not.toHaveBeenCalled();
  });

  test("does nothing when stored guidelines hash matches canonical", async () => {
    mockReadAiConfig.mockResolvedValue({
      ...baseConfig,
      guidelinesHash: "same-hash",
    });

    await checkAiFilesStaleness(
      "same-hash",
      null,
      dummyProjectDir,
      dummyConvexDir,
    );

    expect(mockLogMessage).not.toHaveBeenCalled();
  });

  test("logs nag message when guidelines hash is stale", async () => {
    mockReadAiConfig.mockResolvedValue({
      ...baseConfig,
      guidelinesHash: "old-hash",
    });

    await checkAiFilesStaleness(
      "new-canonical-hash",
      null,
      dummyProjectDir,
      dummyConvexDir,
    );

    expect(mockLogMessage).toHaveBeenCalledWith(
      expect.stringContaining("npx convex ai-files update"),
    );
  });

  test("logs nag message when agent skills SHA is stale", async () => {
    mockReadAiConfig.mockResolvedValue({
      ...baseConfig,
      guidelinesHash: "current-hash",
      agentSkillsSha: "old-sha",
    });

    await checkAiFilesStaleness(
      "current-hash",
      "new-sha",
      dummyProjectDir,
      dummyConvexDir,
    );

    expect(mockLogMessage).toHaveBeenCalledWith(
      expect.stringContaining("npx convex ai-files update"),
    );
  });

  test("does nothing when stored guidelinesHash is null (never written)", async () => {
    mockReadAiConfig.mockResolvedValue(baseConfig);

    await checkAiFilesStaleness(
      "some-hash",
      "some-sha",
      dummyProjectDir,
      dummyConvexDir,
    );

    expect(mockLogMessage).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// updateAiFiles — mock-based.
// ---------------------------------------------------------------------------

describe("updateAiFiles", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockFetchAgentSkillsSha.mockResolvedValue("canonical-sha-abc123");
    mockGetVersion.mockResolvedValue({
      message: null,
      guidelinesHash: null,
      agentSkillsSha: "canonical-sha-abc123",
      disableSkillsCli: false,
    });
  });
  afterEach(() => vi.resetAllMocks());

  test("runs full init and installs skills when no config exists", async () => {
    mockReadAiConfig.mockResolvedValue(null);

    const tmpDir = fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
    const convexDir = path.join(tmpDir, "convex");
    try {
      fs.mkdirSync(convexDir, { recursive: true });
      fs.writeFileSync(path.join(convexDir, "schema.ts"), "");

      mockDownloadGuidelines.mockResolvedValue("guidelines content");

      await updateAiFiles(tmpDir, convexDir);

      expect(
        fs.existsSync(
          path.join(convexDir, "_generated", "ai", "guidelines.md"),
        ),
      ).toBe(true);

      const { default: cp } = await import("child_process");
      const spawnCalls = vi.mocked(cp.spawn).mock.calls;
      const addCall = spawnCalls.find(
        (c) => Array.isArray(c[1]) && c[1].includes("add"),
      );
      expect(addCall).toBeDefined();
    } finally {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });

  test("reports up to date when guidelines hash already matches", async () => {
    mockDownloadGuidelines.mockResolvedValue("guidelines content");
    const tmpDir = fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
    const convexDir = path.join(tmpDir, "convex");

    const { hashSha256 } = await import("../utils/hash.js");
    const realHash = hashSha256("guidelines content");

    mockReadAiConfig.mockResolvedValue({
      ...baseConfig,
      guidelinesHash: realHash,
      agentSkillsSha: "canonical-sha-abc123",
    });

    try {
      await updateAiFiles(tmpDir, convexDir);

      expect(mockLogMessage).toHaveBeenCalledWith(
        expect.stringContaining("already up to date"),
      );
      expect(mockCaptureException).not.toHaveBeenCalled();
    } finally {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });

  test("stores canonical agentSkillsSha and skill names after successful install", async () => {
    const tmpDir = fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
    const convexDir = path.join(tmpDir, "convex");
    try {
      for (const [dir, name] of [
        ["convex_migration_helper", "migration-helper"],
        ["convex_schema_builder", "schema-builder"],
      ]) {
        const skillDir = path.join(tmpDir, ".agents", "skills", dir);
        fs.mkdirSync(skillDir, { recursive: true });
        fs.writeFileSync(
          path.join(skillDir, "SKILL.md"),
          `---\nname: ${name}\ndescription: test\n---\n`,
          "utf8",
        );
      }

      mockDownloadGuidelines.mockResolvedValue(null);
      mockReadAiConfig.mockResolvedValue({
        ...baseConfig,
        agentSkillsSha: "old-sha",
      });

      await updateAiFiles(tmpDir, convexDir);

      expect(mockWriteAiConfig).toHaveBeenCalledWith(
        expect.objectContaining({
          agentSkillsSha: "canonical-sha-abc123",
          installedSkillNames: ["migration-helper", "schema-builder"],
        }),
        expect.anything(),
        expect.anything(),
      );
    } finally {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });

  test("update does not clear disableStalenessMessage when set true", async () => {
    const tmpDir = fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
    const convexDir = path.join(tmpDir, "convex");
    try {
      mockReadAiConfig.mockResolvedValue({
        ...baseConfig,
        disableStalenessMessage: true,
      });
      mockDownloadGuidelines.mockResolvedValue(null);

      await updateAiFiles(tmpDir, convexDir);

      expect(mockWriteAiConfig).toHaveBeenCalledWith(
        expect.objectContaining({ disableStalenessMessage: true }),
        expect.anything(),
        expect.anything(),
      );
    } finally {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });

  test("update recreates convex/_generated/ai when only disable config exists", async () => {
    const tmpDir = fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
    const convexDir = path.join(tmpDir, "convex");
    try {
      fs.mkdirSync(convexDir, { recursive: true });
      fs.writeFileSync(path.join(convexDir, "schema.ts"), "");
      fs.writeFileSync(path.join(tmpDir, "convex.json"), "{}");
      mockReadAiConfig.mockResolvedValue({
        ...baseConfig,
        disableStalenessMessage: true,
        guidelinesHash: null,
      });
      mockDownloadGuidelines.mockResolvedValue("fresh guidelines");

      await updateAiFiles(tmpDir, convexDir);

      expect(fs.existsSync(path.join(convexDir, "_generated", "ai"))).toBe(
        true,
      );
      expect(
        fs.existsSync(
          path.join(convexDir, "_generated", "ai", "guidelines.md"),
        ),
      ).toBe(true);
      expect(mockWriteAiConfig).toHaveBeenCalledWith(
        expect.objectContaining({
          disableStalenessMessage: true,
          guidelinesHash: expect.any(String),
        }),
        tmpDir,
        expect.anything(),
      );
    } finally {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });

  test("logs warning when guidelines download is unavailable", async () => {
    const tmpDir = fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
    const convexDir = path.join(tmpDir, "convex");
    try {
      mockReadAiConfig.mockResolvedValue(baseConfig);
      mockDownloadGuidelines.mockResolvedValue(null);

      await updateAiFiles(tmpDir, convexDir);

      expect(mockLogMessage).toHaveBeenCalledWith(
        expect.stringContaining("Could not download Convex AI guidelines"),
      );
    } finally {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });

  test("skips skills install when server kill switch is enabled", async () => {
    const tmpDir = fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
    const convexDir = path.join(tmpDir, "convex");
    try {
      mockReadAiConfig.mockResolvedValue(baseConfig);
      mockDownloadGuidelines.mockResolvedValue("guidelines content");
      mockGetVersion.mockResolvedValue({
        message: null,
        guidelinesHash: null,
        agentSkillsSha: null,
        disableSkillsCli: true,
      });

      await updateAiFiles(tmpDir, convexDir);

      const { default: cp } = await import("child_process");
      const spawnCalls = vi.mocked(cp.spawn).mock.calls;
      const addCall = spawnCalls.find(
        (c) => Array.isArray(c[1]) && c[1].includes("add"),
      );
      expect(addCall).toBeUndefined();
      expect(mockLogMessage).toHaveBeenCalledWith(
        expect.stringContaining("Agent skills are temporarily disabled."),
      );
    } finally {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });
});

// ---------------------------------------------------------------------------
// removeAiFiles — tested with real temp directories.
// ---------------------------------------------------------------------------

describe("removeAiFiles", () => {
  let tmpDir: string;
  let convexDir: string;

  beforeEach(() => {
    vi.clearAllMocks();
    mockGetVersion.mockResolvedValue({
      message: null,
      guidelinesHash: null,
      agentSkillsSha: "canonical-sha-abc123",
      disableSkillsCli: false,
    });
    tmpDir = fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
    convexDir = path.join(tmpDir, "convex");
    fs.mkdirSync(path.join(convexDir, "_generated", "ai"), {
      recursive: true,
    });
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
    vi.resetAllMocks();
  });

  function writeConfig(override: Partial<typeof baseConfig> = {}) {
    const config = { ...baseConfig, ...override };
    fs.writeFileSync(
      path.join(tmpDir, "convex", "_generated", "ai", "ai-files.state.json"),
      JSON.stringify(config, null, 2) + "\n",
      "utf8",
    );
  }

  test("logs nothing-to-remove when no config exists", async () => {
    // No config file written — readAiConfig returns null.
    mockReadAiConfig.mockResolvedValue(null);

    await removeAiFiles(tmpDir, convexDir);

    expect(mockLogMessage).toHaveBeenCalledWith(
      expect.stringContaining("nothing to remove"),
    );
  });

  test("deletes AGENTS.md if stripping the Convex section leaves it empty", async () => {
    writeConfig();
    mockReadAiConfig.mockResolvedValue(baseConfig);

    const agentsMdContent = `${AGENTS_MD_START_MARKER}\n## Convex\nGuidelines.\n${AGENTS_MD_END_MARKER}\n`;
    fs.writeFileSync(path.join(tmpDir, "AGENTS.md"), agentsMdContent, "utf8");

    await removeAiFiles(tmpDir, convexDir);

    expect(fs.existsSync(path.join(tmpDir, "AGENTS.md"))).toBe(false);
  });

  test("strips Convex section from AGENTS.md", async () => {
    writeConfig();
    mockReadAiConfig.mockResolvedValue(baseConfig);

    const agentsMdContent =
      `# My project\n\n` +
      `${AGENTS_MD_START_MARKER}\n## Convex\nGuidelines.\n${AGENTS_MD_END_MARKER}\n\n` +
      `# After\n`;
    fs.writeFileSync(path.join(tmpDir, "AGENTS.md"), agentsMdContent, "utf8");

    await removeAiFiles(tmpDir, convexDir);

    const result = fs.readFileSync(path.join(tmpDir, "AGENTS.md"), "utf8");
    expect(result).toContain("# My project");
    expect(result).toContain("# After");
    expect(result).not.toContain(AGENTS_MD_START_MARKER);
    expect(result).not.toContain("## Convex");
  });

  test("deletes CLAUDE.md when it only contains the managed section", async () => {
    writeConfig();
    mockReadAiConfig.mockResolvedValue(baseConfig);
    const managed = `${CLAUDE_MD_START_MARKER}\n## Convex\nRead guidelines.\n${CLAUDE_MD_END_MARKER}\n`;
    fs.writeFileSync(path.join(tmpDir, "CLAUDE.md"), managed, "utf8");

    await removeAiFiles(tmpDir, convexDir);

    expect(fs.existsSync(path.join(tmpDir, "CLAUDE.md"))).toBe(false);
  });

  test("leaves CLAUDE.md when it has no managed markers", async () => {
    writeConfig();
    mockReadAiConfig.mockResolvedValue(baseConfig);

    fs.writeFileSync(path.join(tmpDir, "CLAUDE.md"), "User content\n", "utf8");

    await removeAiFiles(tmpDir, convexDir);

    expect(fs.existsSync(path.join(tmpDir, "CLAUDE.md"))).toBe(true);
  });

  test("strips only the Convex section from CLAUDE.md", async () => {
    writeConfig();
    mockReadAiConfig.mockResolvedValue(baseConfig);
    const managed = `${CLAUDE_MD_START_MARKER}\n## Convex\nRead guidelines.\n${CLAUDE_MD_END_MARKER}`;
    fs.writeFileSync(
      path.join(tmpDir, "CLAUDE.md"),
      `# User header\n\n${managed}\n\n# User footer\n`,
      "utf8",
    );

    await removeAiFiles(tmpDir, convexDir);

    const content = fs.readFileSync(path.join(tmpDir, "CLAUDE.md"), "utf8");
    expect(content).toContain("# User header");
    expect(content).toContain("# User footer");
    expect(content).not.toContain(CLAUDE_MD_START_MARKER);
    expect(content).not.toContain("Read guidelines.");
  });

  test("leaves CLAUDE.md alone when it has no managed markers (legacy)", async () => {
    mockReadAiConfig.mockResolvedValue({
      ...baseConfig,
      claudeMdHash: "some-hash",
    });

    fs.writeFileSync(
      path.join(tmpDir, "CLAUDE.md"),
      "My custom CLAUDE.md content\n",
      "utf8",
    );

    await removeAiFiles(tmpDir, convexDir);

    expect(fs.existsSync(path.join(tmpDir, "CLAUDE.md"))).toBe(true);
    expect(fs.readFileSync(path.join(tmpDir, "CLAUDE.md"), "utf8")).toBe(
      "My custom CLAUDE.md content\n",
    );
  });

  test("calls skills remove for each tracked skill name", async () => {
    const skillNames = ["migration-helper", "schema-builder"];
    mockReadAiConfig.mockResolvedValue({
      ...baseConfig,
      installedSkillNames: skillNames,
    });

    await removeAiFiles(tmpDir, convexDir);

    // child_process.spawn should have been called with the skill names.
    const { default: cp } = await import("child_process");
    const spawnCalls = vi.mocked(cp.spawn).mock.calls;
    const removeCall = spawnCalls.find(
      (c) => Array.isArray(c[1]) && c[1].includes("remove"),
    );
    expect(removeCall).toBeDefined();
    expect(removeCall![1]).toContain("migration-helper");
    expect(removeCall![1]).toContain("schema-builder");
  });

  test("deletes skills-lock.json if it becomes empty after removing our skills", async () => {
    const skillNames = ["migration-helper"];
    writeConfig({ installedSkillNames: skillNames });
    mockReadAiConfig.mockResolvedValue({
      ...baseConfig,
      installedSkillNames: skillNames,
    });

    const lockfileContent = {
      version: 1,
      skills: {
        "migration-helper": { source: "test" },
      },
    };
    fs.writeFileSync(
      path.join(tmpDir, "skills-lock.json"),
      JSON.stringify(lockfileContent),
      "utf8",
    );

    await removeAiFiles(tmpDir, convexDir);

    expect(fs.existsSync(path.join(tmpDir, "skills-lock.json"))).toBe(false);
  });

  test("preserves skills-lock.json if it contains other skills", async () => {
    const skillNames = ["migration-helper"];
    writeConfig({ installedSkillNames: skillNames });
    mockReadAiConfig.mockResolvedValue({
      ...baseConfig,
      installedSkillNames: skillNames,
    });

    const lockfileContent = {
      version: 1,
      skills: {
        "migration-helper": { source: "test" },
        "some-other-skill": { source: "other" },
      },
    };
    fs.writeFileSync(
      path.join(tmpDir, "skills-lock.json"),
      JSON.stringify(lockfileContent),
      "utf8",
    );

    await removeAiFiles(tmpDir, convexDir);

    expect(fs.existsSync(path.join(tmpDir, "skills-lock.json"))).toBe(true);
  });

  test("skips skills remove when server kill switch is enabled", async () => {
    const skillNames = ["migration-helper"];
    mockReadAiConfig.mockResolvedValue({
      ...baseConfig,
      installedSkillNames: skillNames,
    });
    mockGetVersion.mockResolvedValue({
      message: null,
      guidelinesHash: null,
      agentSkillsSha: null,
      disableSkillsCli: true,
    });

    await removeAiFiles(tmpDir, convexDir);

    const { default: cp } = await import("child_process");
    const spawnCalls = vi.mocked(cp.spawn).mock.calls;
    const removeCall = spawnCalls.find(
      (c) => Array.isArray(c[1]) && c[1].includes("remove"),
    );
    expect(removeCall).toBeUndefined();
    expect(mockLogMessage).toHaveBeenCalledWith(
      expect.stringContaining("Agent skills are temporarily disabled."),
    );
  });

  test("does NOT write a disabled config after plain remove", async () => {
    writeConfig();
    mockReadAiConfig.mockResolvedValue(baseConfig);

    await removeAiFiles(tmpDir, convexDir);

    // removeAiFiles should not call writeAiConfig — that is disableAiFiles's job.
    expect(mockWriteAiConfig).not.toHaveBeenCalled();
  });

  test("disableAiFiles writes disableStalenessMessage=true without removing files", async () => {
    writeConfig({ guidelinesHash: null });
    mockReadAiConfig.mockResolvedValue(baseConfig);

    fs.writeFileSync(
      path.join(convexDir, "_generated", "ai", "guidelines.md"),
      "guidelines content",
      "utf8",
    );

    await disableAiFiles(tmpDir, convexDir);

    expect(mockWriteAiConfig).toHaveBeenCalledWith(
      expect.objectContaining({ disableStalenessMessage: true }),
      expect.any(String),
      expect.any(String),
      expect.objectContaining({ persistDisabledPreference: "always" }),
    );
    expect(
      fs.existsSync(path.join(convexDir, "_generated", "ai", "guidelines.md")),
    ).toBe(true);
  });

  test("disableAiFiles writes config to project root, not convex dir", async () => {
    mockReadAiConfig.mockResolvedValue(null);

    await disableAiFiles(tmpDir, convexDir);

    expect(mockWriteAiConfig).toHaveBeenCalledWith(
      expect.objectContaining({ disableStalenessMessage: true }),
      tmpDir,
      expect.any(String),
      expect.objectContaining({ persistDisabledPreference: "always" }),
    );
  });
});

// ---------------------------------------------------------------------------
// writeDisabledAiConfig - callers pass convexDir, not projectDir.
// ---------------------------------------------------------------------------

describe("writeDisabledAiConfig", () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(() => vi.resetAllMocks());

  test("writes disable config when given convex dir path", async () => {
    const tmpDir = fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
    try {
      const convexDir = path.join(tmpDir, "convex");
      fs.mkdirSync(convexDir, { recursive: true });

      await writeDisabledAiConfig(convexDir);

      expect(mockWriteAiConfig).toHaveBeenCalledWith(
        expect.objectContaining({ disableStalenessMessage: true }),
        tmpDir,
        convexDir,
        expect.objectContaining({ persistDisabledPreference: "always" }),
      );
    } finally {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });
});

// ---------------------------------------------------------------------------
// statusAiFiles — mock-based.
// ---------------------------------------------------------------------------

describe("statusAiFiles", () => {
  const dummyProjectDir = "/tmp/test-project";
  const dummyConvexDir = "/tmp/test-project/convex";

  beforeEach(() => {
    vi.clearAllMocks();
    mockGetVersion.mockResolvedValue({
      message: null,
      guidelinesHash: "canonical-guidelines-hash",
      agentSkillsSha: "canonical-skills-sha",
      disableSkillsCli: false,
    });
  });
  afterEach(() => vi.resetAllMocks());

  test("reports not installed when config is null", async () => {
    mockReadAiConfig.mockResolvedValue(null);

    await statusAiFiles(dummyProjectDir, dummyConvexDir);

    expect(mockLogMessage).toHaveBeenCalledWith(
      expect.stringContaining("not installed"),
    );
    expect(mockLogMessage).toHaveBeenCalledWith(
      expect.stringContaining("npx convex ai-files install"),
    );
  });

  test("reports disabled when config has disableStalenessMessage=true", async () => {
    mockReadAiConfig.mockResolvedValue({
      ...baseConfig,
      disableStalenessMessage: true,
    });

    await statusAiFiles(dummyProjectDir, dummyConvexDir);

    expect(mockLogMessage).toHaveBeenCalledWith(
      expect.stringContaining("disabled"),
    );
    expect(mockLogMessage).toHaveBeenCalledWith(
      expect.stringContaining("npx convex ai-files enable"),
    );
  });

  test("reports enabled when config exists and disableStalenessMessage=false", async () => {
    mockReadAiConfig.mockResolvedValue(baseConfig);

    await statusAiFiles(dummyProjectDir, dummyConvexDir);

    expect(mockLogMessage).toHaveBeenCalledWith(
      expect.stringContaining("enabled"),
    );
  });

  test("reports guidelines as up to date when hash matches canonical", async () => {
    const tmpDir = fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
    const convexDir = path.join(tmpDir, "convex");
    try {
      const { hashSha256 } = await import("../utils/hash.js");
      const content = "guidelines content";
      const hash = hashSha256(content);

      fs.mkdirSync(path.join(convexDir, "_generated", "ai"), {
        recursive: true,
      });
      fs.writeFileSync(
        path.join(convexDir, "_generated", "ai", "guidelines.md"),
        content,
        "utf8",
      );

      mockReadAiConfig.mockResolvedValue({
        ...baseConfig,
        guidelinesHash: hash,
      });
      mockGetVersion.mockResolvedValue({
        message: null,
        guidelinesHash: hash,
        agentSkillsSha: null,
        disableSkillsCli: false,
      });

      await statusAiFiles(tmpDir, convexDir);

      const calls = mockLogMessage.mock.calls.map((c) => c[0]);
      expect(calls.some((m) => /guidelines\.md.*up to date/.test(m))).toBe(
        true,
      );
    } finally {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });

  test("reports guidelines as out of date when hash differs from canonical", async () => {
    const tmpDir = fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
    const convexDir = path.join(tmpDir, "convex");
    try {
      const { hashSha256 } = await import("../utils/hash.js");
      const content = "old guidelines content";

      fs.mkdirSync(path.join(convexDir, "_generated", "ai"), {
        recursive: true,
      });
      fs.writeFileSync(
        path.join(convexDir, "_generated", "ai", "guidelines.md"),
        content,
        "utf8",
      );

      mockReadAiConfig.mockResolvedValue({
        ...baseConfig,
        guidelinesHash: hashSha256(content),
      });
      mockGetVersion.mockResolvedValue({
        message: null,
        guidelinesHash: "new-canonical-hash",
        agentSkillsSha: null,
        disableSkillsCli: false,
      });

      await statusAiFiles(tmpDir, convexDir);

      expect(mockLogMessage).toHaveBeenCalledWith(
        expect.stringContaining("out of date"),
      );
      expect(mockLogMessage).toHaveBeenCalledWith(
        expect.stringContaining("npx convex ai-files update"),
      );
    } finally {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });

  test("reports guidelines as locally modified when disk hash differs from stored", async () => {
    const tmpDir = fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
    const convexDir = path.join(tmpDir, "convex");
    try {
      const { hashSha256 } = await import("../utils/hash.js");

      fs.mkdirSync(path.join(convexDir, "_generated", "ai"), {
        recursive: true,
      });
      fs.writeFileSync(
        path.join(convexDir, "_generated", "ai", "guidelines.md"),
        "user-modified content",
        "utf8",
      );

      mockReadAiConfig.mockResolvedValue({
        ...baseConfig,
        guidelinesHash: hashSha256("original content"),
      });

      await statusAiFiles(tmpDir, convexDir);

      expect(mockLogMessage).toHaveBeenCalledWith(
        expect.stringContaining("modified locally"),
      );
    } finally {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });

  test("reports agent skills as out of date when SHA differs from canonical", async () => {
    mockReadAiConfig.mockResolvedValue({
      ...baseConfig,
      installedSkillNames: ["migration-helper"],
      agentSkillsSha: "old-sha",
    });
    mockGetVersion.mockResolvedValue({
      message: null,
      guidelinesHash: null,
      agentSkillsSha: "new-sha",
      disableSkillsCli: false,
    });

    await statusAiFiles(dummyProjectDir, dummyConvexDir);

    expect(mockLogMessage).toHaveBeenCalledWith(
      expect.stringContaining("out of date"),
    );
    expect(mockLogMessage).toHaveBeenCalledWith(
      expect.stringContaining("npx convex ai-files update"),
    );
  });

  test("skips staleness check when network is unavailable", async () => {
    mockReadAiConfig.mockResolvedValue({
      ...baseConfig,
      guidelinesHash: "old-hash",
      agentSkillsSha: "old-sha",
      installedSkillNames: ["migration-helper"],
    });
    mockGetVersion.mockResolvedValue(null);

    await statusAiFiles(dummyProjectDir, dummyConvexDir);

    const calls = mockLogMessage.mock.calls.map((c) => c[0]);
    expect(calls.some((m) => /out of date/.test(m))).toBe(false);
  });

  test("reports skills with names when installed", async () => {
    mockReadAiConfig.mockResolvedValue({
      ...baseConfig,
      installedSkillNames: ["migration-helper", "schema-builder"],
      agentSkillsSha: "canonical-skills-sha",
    });

    await statusAiFiles(dummyProjectDir, dummyConvexDir);

    expect(mockLogMessage).toHaveBeenCalledWith(
      expect.stringContaining("migration-helper"),
    );
    expect(mockLogMessage).toHaveBeenCalledWith(
      expect.stringContaining("schema-builder"),
    );
  });

  test("reports CLAUDE.md section as missing when file exists without markers", async () => {
    const tmpDir = fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
    const convexDir = path.join(tmpDir, "convex");
    try {
      fs.writeFileSync(
        path.join(tmpDir, "CLAUDE.md"),
        "User content\n",
        "utf8",
      );
      mockReadAiConfig.mockResolvedValue(baseConfig);

      await statusAiFiles(tmpDir, convexDir);

      expect(mockLogMessage).toHaveBeenCalledWith(
        expect.stringContaining("CLAUDE.md: no Convex section present"),
      );
    } finally {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });
});
