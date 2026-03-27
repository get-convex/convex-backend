import { describe, test, expect, vi, beforeEach, afterEach } from "vitest";
import * as Sentry from "@sentry/node";
import { promises as fs } from "fs";
import {
  aiFilesStateSchema,
  hasAiFilesConfig,
  readAiConfig,
  writeAiConfig,
  writeAiEnabledToProjectConfig,
} from "./config.js";

vi.mock("@sentry/node", () => ({
  captureException: vi.fn(),
  captureMessage: vi.fn(),
}));

vi.mock("fs", () => ({
  promises: {
    readFile: vi.fn(),
    writeFile: vi.fn(),
  },
}));

const mockFs = vi.mocked(fs);
const mockCaptureException = vi.mocked(Sentry.captureException);

const dummyProjectDir = "/tmp/test-project";
const dummyConvexDir = "/tmp/test-project/convex";

describe("aiFilesStateSchema", () => {
  test("accepts a fully populated valid state object", () => {
    const result = aiFilesStateSchema.safeParse({
      guidelinesHash: "abc123",
      agentsMdSectionHash: "def456",
      claudeMdHash: "ghi789",
      agentSkillsSha: "deadbeef",
      installedSkillNames: ["convex-migrations", "convex-perf-review"],
    });
    expect(result.success).toBe(true);
  });

  test("accepts null hashes", () => {
    const result = aiFilesStateSchema.safeParse({
      guidelinesHash: null,
      agentsMdSectionHash: null,
      claudeMdHash: null,
      agentSkillsSha: null,
    });
    expect(result.success).toBe(true);
  });

  test("applies default for installedSkillNames when absent", () => {
    const result = aiFilesStateSchema.safeParse({
      guidelinesHash: "abc",
      agentsMdSectionHash: null,
      claudeMdHash: null,
      agentSkillsSha: null,
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.installedSkillNames).toEqual([]);
    }
  });

  test("rejects a number where a string hash is expected", () => {
    const result = aiFilesStateSchema.safeParse({
      guidelinesHash: 123,
      agentsMdSectionHash: null,
      claudeMdHash: null,
      agentSkillsSha: null,
    });
    expect(result.success).toBe(false);
  });

  test("rejects missing required fields", () => {
    const result = aiFilesStateSchema.safeParse({
      guidelinesHash: "abc",
    });
    expect(result.success).toBe(false);
  });
});

describe("readAiConfig", () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(() => vi.resetAllMocks());

  test("returns null when the file does not exist", async () => {
    mockFs.readFile.mockRejectedValue(
      Object.assign(new Error("ENOENT"), { code: "ENOENT" }),
    );

    const result = await readAiConfig({
      projectDir: dummyProjectDir,
      convexDir: dummyConvexDir,
    });

    expect(result).toBeNull();
    expect(mockCaptureException).not.toHaveBeenCalled();
  });

  test("returns parsed state with enabled=true when convex.json is missing", async () => {
    mockFs.readFile
      .mockRejectedValueOnce(
        Object.assign(new Error("ENOENT"), { code: "ENOENT" }),
      )
      .mockResolvedValueOnce(
        JSON.stringify({
          guidelinesHash: "abc",
          agentsMdSectionHash: "def",
          claudeMdHash: null,
          agentSkillsSha: null,
        }),
      );

    const result = await readAiConfig({
      projectDir: dummyProjectDir,
      convexDir: dummyConvexDir,
    });

    expect(result).toEqual({
      guidelinesHash: "abc",
      agentsMdSectionHash: "def",
      claudeMdHash: null,
      agentSkillsSha: null,
      installedSkillNames: [],
      enabled: true,
    });
  });

  test("returns empty disabled config when state file is missing and convex.json disables nags", async () => {
    mockFs.readFile
      .mockResolvedValueOnce(
        JSON.stringify({ aiFiles: { disableStalenessMessage: true } }),
      )
      .mockRejectedValueOnce(
        Object.assign(new Error("ENOENT"), { code: "ENOENT" }),
      );

    const result = await readAiConfig({
      projectDir: dummyProjectDir,
      convexDir: dummyConvexDir,
    });

    expect(result).toEqual({
      guidelinesHash: null,
      agentsMdSectionHash: null,
      claudeMdHash: null,
      agentSkillsSha: null,
      installedSkillNames: [],
      enabled: false,
    });
  });

  test("returns null and captures exception when JSON is invalid", async () => {
    mockFs.readFile
      .mockResolvedValueOnce(
        JSON.stringify({ aiFiles: { disableStalenessMessage: false } }),
      )
      .mockResolvedValueOnce("not valid json {{{}");

    const result = await readAiConfig({
      projectDir: dummyProjectDir,
      convexDir: dummyConvexDir,
    });

    expect(result).toBeNull();
    expect(mockCaptureException).toHaveBeenCalledWith(expect.any(Error));
  });

  test("returns null and captures exception when schema validation fails", async () => {
    mockFs.readFile
      .mockResolvedValueOnce(
        JSON.stringify({ aiFiles: { disableStalenessMessage: false } }),
      )
      .mockResolvedValueOnce(
        JSON.stringify({
          guidelinesHash: 99,
          agentsMdSectionHash: null,
        }),
      );

    const result = await readAiConfig({
      projectDir: dummyProjectDir,
      convexDir: dummyConvexDir,
    });

    expect(result).toBeNull();
    expect(mockCaptureException).toHaveBeenCalledWith(expect.any(Error));
  });

  test("reads enabled=false from legacy disableStalenessMessage field for backward compat", async () => {
    const stored = {
      guidelinesHash: "abc",
      agentsMdSectionHash: "def",
      claudeMdHash: null,
      agentSkillsSha: null,
    };
    mockFs.readFile
      .mockResolvedValueOnce(
        JSON.stringify({ aiFiles: { disableStalenessMessage: true } }),
      )
      .mockResolvedValueOnce(JSON.stringify(stored));

    const result = await readAiConfig({
      projectDir: dummyProjectDir,
      convexDir: dummyConvexDir,
    });

    expect(result).toEqual({
      ...stored,
      installedSkillNames: [],
      enabled: false,
    });
  });

  test("enabled: true takes precedence over legacy disableStalenessMessage: true", async () => {
    const stored = {
      guidelinesHash: "abc",
      agentsMdSectionHash: "def",
      claudeMdHash: null,
      agentSkillsSha: null,
    };
    mockFs.readFile
      .mockResolvedValueOnce(
        JSON.stringify({
          aiFiles: { enabled: true, disableStalenessMessage: true },
        }),
      )
      .mockResolvedValueOnce(JSON.stringify(stored));

    const result = await readAiConfig({
      projectDir: dummyProjectDir,
      convexDir: dummyConvexDir,
    });

    expect(result).toEqual({
      ...stored,
      installedSkillNames: [],
      enabled: true,
    });
  });
});

describe("hasAiFilesConfig", () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(() => vi.resetAllMocks());

  test("returns false when neither convex.json nor state file exists", async () => {
    mockFs.readFile.mockRejectedValue(
      Object.assign(new Error("ENOENT"), { code: "ENOENT" }),
    );

    const result = await hasAiFilesConfig({
      projectDir: dummyProjectDir,
      convexDir: dummyConvexDir,
    });

    expect(result).toBe(false);
    expect(mockCaptureException).not.toHaveBeenCalled();
  });

  test("returns true when convex.json disables staleness messages", async () => {
    mockFs.readFile.mockResolvedValueOnce(
      JSON.stringify({ aiFiles: { disableStalenessMessage: true } }),
    );

    const result = await hasAiFilesConfig({
      projectDir: dummyProjectDir,
      convexDir: dummyConvexDir,
    });

    expect(result).toBe(true);
  });

  test("returns true when a valid state file exists", async () => {
    mockFs.readFile
      .mockRejectedValueOnce(
        Object.assign(new Error("ENOENT"), { code: "ENOENT" }),
      )
      .mockResolvedValueOnce(
        JSON.stringify({
          guidelinesHash: "abc",
          agentsMdSectionHash: "def",
          claudeMdHash: null,
          agentSkillsSha: null,
        }),
      );

    const result = await hasAiFilesConfig({
      projectDir: dummyProjectDir,
      convexDir: dummyConvexDir,
    });

    expect(result).toBe(true);
  });

  test("returns false and captures exception when the state file is invalid", async () => {
    mockFs.readFile
      .mockRejectedValueOnce(
        Object.assign(new Error("ENOENT"), { code: "ENOENT" }),
      )
      .mockResolvedValueOnce("not valid json {{{}");

    const result = await hasAiFilesConfig({
      projectDir: dummyProjectDir,
      convexDir: dummyConvexDir,
    });

    expect(result).toBe(false);
    expect(mockCaptureException).toHaveBeenCalledWith(expect.any(Error));
  });
});

describe("writeAiConfig", () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(() => vi.resetAllMocks());

  test("writes state file and does not persist enabled=true by default", async () => {
    mockFs.writeFile.mockResolvedValue(undefined);

    const config = {
      enabled: true,
      guidelinesHash: "abc",
      agentsMdSectionHash: "def",
      claudeMdHash: null,
      agentSkillsSha: null,
      installedSkillNames: ["convex-migrations"],
    };

    await writeAiConfig({
      config,
      projectDir: dummyProjectDir,
      convexDir: dummyConvexDir,
    });

    expect(mockFs.writeFile).toHaveBeenCalledTimes(1);
    expect(mockFs.writeFile).toHaveBeenCalledWith(
      expect.stringContaining("ai-files.state.json"),
      JSON.stringify(
        {
          guidelinesHash: "abc",
          agentsMdSectionHash: "def",
          claudeMdHash: null,
          agentSkillsSha: null,
          installedSkillNames: ["convex-migrations"],
        },
        null,
        2,
      ) + "\n",
      "utf8",
    );
  });

  test("persists enabled=true when requested explicitly", async () => {
    mockFs.writeFile.mockResolvedValue(undefined);
    mockFs.readFile.mockResolvedValue("{}");

    await writeAiConfig({
      config: {
        enabled: true,
        guidelinesHash: null,
        agentsMdSectionHash: null,
        claudeMdHash: null,
        agentSkillsSha: null,
        installedSkillNames: [],
      },
      projectDir: dummyProjectDir,
      convexDir: dummyConvexDir,
      options: { persistEnabledPreference: "always" },
    });

    expect(mockFs.writeFile).toHaveBeenNthCalledWith(
      2,
      expect.stringContaining("convex.json"),
      JSON.stringify(
        {
          $schema: "node_modules/convex/schemas/convex.schema.json",
          aiFiles: { enabled: true },
        },
        null,
        2,
      ) + "\n",
      "utf8",
    );
  });

  test("persists enabled=false to convex.json by default", async () => {
    mockFs.writeFile.mockResolvedValue(undefined);
    mockFs.readFile.mockResolvedValue("{}");

    await writeAiConfig({
      config: {
        enabled: false,
        guidelinesHash: null,
        agentsMdSectionHash: null,
        claudeMdHash: null,
        agentSkillsSha: null,
        installedSkillNames: [],
      },
      projectDir: dummyProjectDir,
      convexDir: dummyConvexDir,
    });

    expect(mockFs.writeFile).toHaveBeenCalledTimes(2);
    expect(mockFs.writeFile).toHaveBeenNthCalledWith(
      2,
      expect.stringContaining("convex.json"),
      expect.stringContaining('"enabled": false'),
      "utf8",
    );
  });

  test("never writes to convex.json when persistEnabledPreference is 'never'", async () => {
    mockFs.writeFile.mockResolvedValue(undefined);

    await writeAiConfig({
      config: {
        enabled: false,
        guidelinesHash: null,
        agentsMdSectionHash: null,
        claudeMdHash: null,
        agentSkillsSha: null,
        installedSkillNames: [],
      },
      projectDir: dummyProjectDir,
      convexDir: dummyConvexDir,
      options: { persistEnabledPreference: "never" },
    });

    expect(mockFs.writeFile).toHaveBeenCalledTimes(1);
    expect(mockFs.writeFile).toHaveBeenCalledWith(
      expect.stringContaining("ai-files.state.json"),
      expect.any(String),
      "utf8",
    );
  });
});

describe("writeAiEnabledToProjectConfig", () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(() => vi.resetAllMocks());

  test("writes enabled=false to convex.json and drops legacy disableStalenessMessage", async () => {
    mockFs.readFile.mockResolvedValue(
      JSON.stringify({ aiFiles: { disableStalenessMessage: true } }),
    );
    mockFs.writeFile.mockResolvedValue(undefined);

    await writeAiEnabledToProjectConfig({
      projectDir: dummyProjectDir,
      enabled: false,
    });

    expect(mockFs.writeFile).toHaveBeenCalledTimes(1);
    const written = mockFs.writeFile.mock.calls[0][1] as string;
    const parsed = JSON.parse(written);
    expect(parsed.aiFiles.enabled).toBe(false);
    expect(parsed.aiFiles.disableStalenessMessage).toBeUndefined();
  });
});
