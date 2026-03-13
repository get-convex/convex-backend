import { describe, test, expect, vi, beforeEach, afterEach } from "vitest";
import * as Sentry from "@sentry/node";
import { promises as fs } from "fs";
import { aiFilesSchema, readAiConfig, writeAiConfig } from "./config.js";

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

describe("aiFilesSchema", () => {
  test("accepts a fully populated valid state object", () => {
    const result = aiFilesSchema.safeParse({
      guidelinesHash: "abc123",
      agentsMdSectionHash: "def456",
      claudeMdHash: "ghi789",
      agentSkillsSha: "deadbeef",
      installedSkillNames: ["convex-migrations", "convex-perf-review"],
    });
    expect(result.success).toBe(true);
  });

  test("accepts null hashes", () => {
    const result = aiFilesSchema.safeParse({
      guidelinesHash: null,
      agentsMdSectionHash: null,
      claudeMdHash: null,
      agentSkillsSha: null,
    });
    expect(result.success).toBe(true);
  });

  test("applies default for installedSkillNames when absent", () => {
    const result = aiFilesSchema.safeParse({
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
    const result = aiFilesSchema.safeParse({
      guidelinesHash: 123,
      agentsMdSectionHash: null,
      claudeMdHash: null,
      agentSkillsSha: null,
    });
    expect(result.success).toBe(false);
  });

  test("rejects missing required fields", () => {
    const result = aiFilesSchema.safeParse({
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

    const result = await readAiConfig(dummyProjectDir, dummyConvexDir);

    expect(result).toBeNull();
    expect(mockCaptureException).not.toHaveBeenCalled();
  });

  test("returns parsed state with disableStalenessMessage=false when convex.json is missing", async () => {
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

    const result = await readAiConfig(dummyProjectDir, dummyConvexDir);

    expect(result).toEqual({
      guidelinesHash: "abc",
      agentsMdSectionHash: "def",
      claudeMdHash: null,
      agentSkillsSha: null,
      installedSkillNames: [],
      disableStalenessMessage: false,
    });
  });

  test("returns empty state when disableStalenessMessage in convex.json and state file is missing", async () => {
    mockFs.readFile
      .mockResolvedValueOnce(
        JSON.stringify({ aiFiles: { disableStalenessMessage: true } }),
      )
      .mockRejectedValueOnce(
        Object.assign(new Error("ENOENT"), { code: "ENOENT" }),
      );

    const result = await readAiConfig(dummyProjectDir, dummyConvexDir);

    expect(result).toEqual({
      guidelinesHash: null,
      agentsMdSectionHash: null,
      claudeMdHash: null,
      agentSkillsSha: null,
      installedSkillNames: [],
      disableStalenessMessage: true,
    });
  });

  test("returns null and captures exception when JSON is invalid", async () => {
    mockFs.readFile
      .mockResolvedValueOnce(
        JSON.stringify({ aiFiles: { disableStalenessMessage: false } }),
      )
      .mockResolvedValueOnce("not valid json {{{}");

    const result = await readAiConfig(dummyProjectDir, dummyConvexDir);

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

    const result = await readAiConfig(dummyProjectDir, dummyConvexDir);

    expect(result).toBeNull();
    expect(mockCaptureException).toHaveBeenCalledWith(expect.any(Error));
  });

  test("returns parsed state and reads disableStalenessMessage from convex.json", async () => {
    const stored = {
      guidelinesHash: "abc",
      agentsMdSectionHash: "def",
      claudeMdHash: null,
      agentSkillsSha: "deadbeef",
    };
    mockFs.readFile
      .mockResolvedValueOnce(
        JSON.stringify({ aiFiles: { disableStalenessMessage: true } }),
      )
      .mockResolvedValueOnce(JSON.stringify(stored));

    const result = await readAiConfig(dummyProjectDir, dummyConvexDir);

    expect(result).toEqual({
      ...stored,
      installedSkillNames: [],
      disableStalenessMessage: true,
    });
  });
});

describe("writeAiConfig", () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(() => vi.resetAllMocks());

  test("writes state file and does not persist disableStalenessMessage=false by default", async () => {
    mockFs.writeFile.mockResolvedValue(undefined);

    const config = {
      disableStalenessMessage: false,
      guidelinesHash: "abc",
      agentsMdSectionHash: "def",
      claudeMdHash: null,
      agentSkillsSha: null,
      installedSkillNames: ["convex-migrations"],
    };

    await writeAiConfig(config, dummyProjectDir, dummyConvexDir);

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

  test("persists disableStalenessMessage=false when requested explicitly", async () => {
    mockFs.writeFile.mockResolvedValue(undefined);
    mockFs.readFile.mockResolvedValue("{}");

    await writeAiConfig(
      {
        disableStalenessMessage: false,
        guidelinesHash: null,
        agentsMdSectionHash: null,
        claudeMdHash: null,
        agentSkillsSha: null,
        installedSkillNames: [],
      },
      dummyProjectDir,
      dummyConvexDir,
      { persistDisabledPreference: "always" },
    );

    expect(mockFs.writeFile).toHaveBeenNthCalledWith(
      2,
      expect.stringContaining("convex.json"),
      JSON.stringify(
        {
          $schema: "node_modules/convex/schemas/convex.schema.json",
          aiFiles: {
            disableStalenessMessage: false,
          },
        },
        null,
        2,
      ) + "\n",
      "utf8",
    );
  });

  test("persists to convex.json by default when disableStalenessMessage is true", async () => {
    mockFs.writeFile.mockResolvedValue(undefined);
    mockFs.readFile.mockResolvedValue("{}");

    await writeAiConfig(
      {
        disableStalenessMessage: true,
        guidelinesHash: null,
        agentsMdSectionHash: null,
        claudeMdHash: null,
        agentSkillsSha: null,
        installedSkillNames: [],
      },
      dummyProjectDir,
      dummyConvexDir,
    );

    expect(mockFs.writeFile).toHaveBeenCalledTimes(2);
    expect(mockFs.writeFile).toHaveBeenNthCalledWith(
      2,
      expect.stringContaining("convex.json"),
      expect.stringContaining('"disableStalenessMessage": true'),
      "utf8",
    );
  });

  test("never writes to convex.json when persistDisabledPreference is 'never'", async () => {
    mockFs.writeFile.mockResolvedValue(undefined);

    await writeAiConfig(
      {
        disableStalenessMessage: true,
        guidelinesHash: null,
        agentsMdSectionHash: null,
        claudeMdHash: null,
        agentSkillsSha: null,
        installedSkillNames: [],
      },
      dummyProjectDir,
      dummyConvexDir,
      { persistDisabledPreference: "never" },
    );

    expect(mockFs.writeFile).toHaveBeenCalledTimes(1);
    expect(mockFs.writeFile).toHaveBeenCalledWith(
      expect.stringContaining("ai-files.state.json"),
      expect.any(String),
      "utf8",
    );
  });
});
