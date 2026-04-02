import { describe, test, expect, vi, beforeEach, afterEach } from "vitest";
import * as Sentry from "@sentry/node";
import { promises as fs } from "fs";
import {
  aiFilesStateSchema,
  attemptReadAiState,
  hasAiState,
  readAiStateOrDefault,
  writeAiState,
} from "./state.js";

vi.mock("@sentry/node", () => ({
  captureException: vi.fn(),
  captureMessage: vi.fn(),
}));

vi.mock("fs", () => ({
  promises: {
    readFile: vi.fn(),
    writeFile: vi.fn(),
    mkdir: vi.fn().mockResolvedValue(undefined),
  },
}));

const mockFs = vi.mocked(fs);
const mockCaptureException = vi.mocked(Sentry.captureException);

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

describe("attemptReadAiState", () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(() => vi.resetAllMocks());

  test("returns no-file when the state file does not exist", async () => {
    mockFs.readFile.mockRejectedValue(
      Object.assign(new Error("ENOENT"), { code: "ENOENT" }),
    );

    const result = await attemptReadAiState(dummyConvexDir);

    expect(result).toEqual({ kind: "no-file" });
    expect(mockCaptureException).not.toHaveBeenCalled();
  });

  test("returns no-file when the state file is empty", async () => {
    mockFs.readFile.mockResolvedValueOnce("");

    const result = await attemptReadAiState(dummyConvexDir);

    expect(result).toEqual({ kind: "no-file" });
    expect(mockCaptureException).not.toHaveBeenCalled();
  });

  test("throws when reading the state file fails for reasons other than not found", async () => {
    const error = Object.assign(new Error("EACCES"), { code: "EACCES" });
    mockFs.readFile.mockRejectedValueOnce(error);

    await expect(attemptReadAiState(dummyConvexDir)).rejects.toBe(error);
    expect(mockCaptureException).not.toHaveBeenCalled();
  });

  test("returns ok with parsed state", async () => {
    mockFs.readFile.mockResolvedValueOnce(
      JSON.stringify({
        guidelinesHash: "abc",
        agentsMdSectionHash: "def",
        claudeMdHash: null,
        agentSkillsSha: null,
      }),
    );

    const result = await attemptReadAiState(dummyConvexDir);

    expect(result).toEqual({
      kind: "ok",
      state: {
        guidelinesHash: "abc",
        agentsMdSectionHash: "def",
        claudeMdHash: null,
        agentSkillsSha: null,
        installedSkillNames: [],
      },
    });
  });

  test("returns parse-error and captures exception when JSON is invalid", async () => {
    mockFs.readFile.mockResolvedValueOnce("not valid json {{{}");

    const result = await attemptReadAiState(dummyConvexDir);

    expect(result.kind).toBe("parse-error");
    expect(mockCaptureException).toHaveBeenCalledWith(expect.any(Error));
  });

  test("returns parse-error and captures exception when schema validation fails", async () => {
    mockFs.readFile.mockResolvedValueOnce(
      JSON.stringify({
        guidelinesHash: 99,
        agentsMdSectionHash: null,
      }),
    );

    const result = await attemptReadAiState(dummyConvexDir);

    expect(result.kind).toBe("parse-error");
    expect(mockCaptureException).toHaveBeenCalledWith(expect.any(Error));
  });
});

describe("readAiStateOrDefault", () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(() => vi.resetAllMocks());

  test("returns default state when no state file exists", async () => {
    mockFs.readFile.mockRejectedValue(
      Object.assign(new Error("ENOENT"), { code: "ENOENT" }),
    );

    const result = await readAiStateOrDefault(dummyConvexDir);

    expect(result).toEqual({
      guidelinesHash: null,
      agentsMdSectionHash: null,
      claudeMdHash: null,
      agentSkillsSha: null,
      installedSkillNames: [],
    });
  });

  test("returns default state on parse error", async () => {
    mockFs.readFile.mockResolvedValueOnce("not valid json {{{}");

    const result = await readAiStateOrDefault(dummyConvexDir);

    expect(result).toEqual({
      guidelinesHash: null,
      agentsMdSectionHash: null,
      claudeMdHash: null,
      agentSkillsSha: null,
      installedSkillNames: [],
    });
    expect(mockCaptureException).toHaveBeenCalled();
  });

  test("returns parsed state when state file exists", async () => {
    const stored = {
      guidelinesHash: "abc",
      agentsMdSectionHash: "def",
      claudeMdHash: null,
      agentSkillsSha: "deadbeef",
    };
    mockFs.readFile.mockResolvedValueOnce(JSON.stringify(stored));

    const result = await readAiStateOrDefault(dummyConvexDir);

    expect(result).toEqual({
      ...stored,
      installedSkillNames: [],
    });
  });
});

describe("hasAiState", () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(() => vi.resetAllMocks());

  test("returns false when state file does not exist", async () => {
    mockFs.readFile.mockRejectedValue(
      Object.assign(new Error("ENOENT"), { code: "ENOENT" }),
    );

    const result = await hasAiState(dummyConvexDir);

    expect(result).toBe(false);
    expect(mockCaptureException).not.toHaveBeenCalled();
  });

  test("returns true when a valid state file exists", async () => {
    mockFs.readFile.mockResolvedValueOnce(
      JSON.stringify({
        guidelinesHash: "abc",
        agentsMdSectionHash: "def",
        claudeMdHash: null,
        agentSkillsSha: null,
      }),
    );

    const result = await hasAiState(dummyConvexDir);

    expect(result).toBe(true);
  });

  test("returns false and captures exception when the state file is invalid", async () => {
    mockFs.readFile.mockResolvedValueOnce("not valid json {{{}");

    const result = await hasAiState(dummyConvexDir);

    expect(result).toBe(false);
    expect(mockCaptureException).toHaveBeenCalledWith(expect.any(Error));
  });
});

describe("writeAiState", () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(() => vi.resetAllMocks());

  test("writes state file", async () => {
    mockFs.writeFile.mockResolvedValue(undefined);

    const state = {
      guidelinesHash: "abc",
      agentsMdSectionHash: "def",
      claudeMdHash: null,
      agentSkillsSha: null,
      installedSkillNames: ["convex-migrations"],
    };

    await writeAiState({ state, convexDir: dummyConvexDir });

    expect(mockFs.writeFile).toHaveBeenCalledTimes(1);
    expect(mockFs.writeFile).toHaveBeenCalledWith(
      expect.stringContaining("ai-files.state.json"),
      JSON.stringify(state, null, 2) + "\n",
      "utf8",
    );
  });
});
