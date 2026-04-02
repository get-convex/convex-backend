import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { checkVersionAndAiFilesStaleness } from "./updates.js";
import { getVersion } from "./versionApi.js";
import { logMessage } from "../../bundler/log.js";
import { checkAiFilesStalenessAndLog } from "./aiFiles/index.js";
import { readProjectConfig } from "./config.js";
import type { Context } from "../../bundler/context.js";

vi.mock("./versionApi.js", () => ({
  getVersion: vi.fn(),
}));

vi.mock("./aiFiles/index.js", () => ({
  checkAiFilesStalenessAndLog: vi.fn(),
  isAiFilesDisabled: vi.fn((aiFilesConfig) =>
    aiFilesConfig?.enabled !== undefined
      ? aiFilesConfig.enabled === false
      : aiFilesConfig?.disableStalenessMessage === true,
  ),
}));

vi.mock("../../bundler/log.js", () => ({
  logMessage: vi.fn(),
}));

vi.mock("./config.js", () => ({
  readProjectConfig: vi.fn(),
}));

const mockGetVersion = vi.mocked(getVersion);
const mockLogMessage = vi.mocked(logMessage);
const mockCheckAiFilesStalenessAndLog = vi.mocked(checkAiFilesStalenessAndLog);
const mockReadProjectConfig = vi.mocked(readProjectConfig);

const fakeCtx = {} as Context;

const okVersion = (overrides?: object) => ({
  kind: "ok" as const,
  data: {
    message: null,
    guidelinesHash: "abc-guidelines-hash",
    agentSkillsSha: "abc-skills-sha",
    disableSkillsCli: false,
    ...overrides,
  },
});

describe("checkVersionAndAiFilesStaleness", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockCheckAiFilesStalenessAndLog.mockResolvedValue(undefined);
    mockReadProjectConfig.mockResolvedValue({
      projectConfig: {
        functions: "convex",
        node: { externalPackages: [] },
        generateCommonJSApi: false,
        codegen: { staticApi: true, staticDataModel: true },
      },
      configPath: "convex.json",
    });
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it("logs version message when server provides one", async () => {
    mockGetVersion.mockResolvedValue(
      okVersion({ message: "New version available: 1.2.3" }),
    );

    await checkVersionAndAiFilesStaleness(fakeCtx);

    expect(mockLogMessage).toHaveBeenCalledWith("New version available: 1.2.3");
  });

  it("does not log when version has no message", async () => {
    mockGetVersion.mockResolvedValue(okVersion());

    await checkVersionAndAiFilesStaleness(fakeCtx);

    expect(mockLogMessage).not.toHaveBeenCalled();
  });

  it("does nothing when getVersion returns error", async () => {
    mockGetVersion.mockResolvedValue({ kind: "error" });

    await checkVersionAndAiFilesStaleness(fakeCtx);

    expect(mockLogMessage).not.toHaveBeenCalled();
    expect(mockCheckAiFilesStalenessAndLog).not.toHaveBeenCalled();
  });

  it("passes hashes and project paths to staleness check", async () => {
    mockGetVersion.mockResolvedValue(
      okVersion({
        guidelinesHash:
          "02e43fc1ff0ee48db8da468f5c7525877d8056fcd56c77d78a166ac447efb91c",
        agentSkillsSha: "abc123def456abc123def456abc123def456abc1",
      }),
    );

    await checkVersionAndAiFilesStaleness(fakeCtx);

    expect(mockCheckAiFilesStalenessAndLog).toHaveBeenCalledWith({
      canonicalGuidelinesHash:
        "02e43fc1ff0ee48db8da468f5c7525877d8056fcd56c77d78a166ac447efb91c",
      canonicalAgentSkillsSha: "abc123def456abc123def456abc123def456abc1",
      projectDir: expect.any(String),
      convexDir: expect.any(String),
    });
  });

  it("passes null hashes when version has none", async () => {
    mockGetVersion.mockResolvedValue(
      okVersion({ guidelinesHash: null, agentSkillsSha: null }),
    );

    await checkVersionAndAiFilesStaleness(fakeCtx);

    expect(mockCheckAiFilesStalenessAndLog).toHaveBeenCalledWith({
      canonicalGuidelinesHash: null,
      canonicalAgentSkillsSha: null,
      projectDir: expect.any(String),
      convexDir: expect.any(String),
    });
  });

  it("skips staleness check when aiFiles.disableStalenessMessage is true", async () => {
    mockGetVersion.mockResolvedValue(okVersion());
    mockReadProjectConfig.mockResolvedValue({
      projectConfig: {
        functions: "convex",
        node: { externalPackages: [] },
        generateCommonJSApi: false,
        codegen: { staticApi: true, staticDataModel: true },
        aiFiles: { disableStalenessMessage: true },
      },
      configPath: "convex.json",
    });

    await checkVersionAndAiFilesStaleness(fakeCtx);

    expect(mockCheckAiFilesStalenessAndLog).not.toHaveBeenCalled();
  });

  it("silently skips staleness check when project config cannot be resolved", async () => {
    mockGetVersion.mockResolvedValue(okVersion());
    mockReadProjectConfig.mockRejectedValue(new Error("no convex.json"));

    await expect(
      checkVersionAndAiFilesStaleness(fakeCtx),
    ).resolves.toBeUndefined();
    expect(mockCheckAiFilesStalenessAndLog).not.toHaveBeenCalled();
  });
});
