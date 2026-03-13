import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { checkVersion } from "./updates.js";
import { getVersion } from "./versionApi.js";
import { logMessage } from "../../bundler/log.js";
import { checkAiFilesStaleness } from "./ai/index.js";
import { readProjectConfig } from "./config.js";
import type { Context } from "../../bundler/context.js";

vi.mock("./versionApi.js", () => ({
  getVersion: vi.fn(),
}));

vi.mock("./ai/index.js", () => ({
  checkAiFilesStaleness: vi.fn(),
}));

vi.mock("../../bundler/log.js", () => ({
  logMessage: vi.fn(),
}));

vi.mock("./config.js", () => ({
  readProjectConfig: vi.fn(),
}));

const mockGetVersion = vi.mocked(getVersion);
const mockLogMessage = vi.mocked(logMessage);
const mockCheckAiFilesStaleness = vi.mocked(checkAiFilesStaleness);
const mockReadProjectConfig = vi.mocked(readProjectConfig);

const fakeCtx = {} as Context;

describe("updates", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockCheckAiFilesStaleness.mockResolvedValue(undefined);
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

  describe("checkVersion", () => {
    it("logs message and passes both hashes to staleness check", async () => {
      const sha = "abc123def456abc123def456abc123def456abc1";
      mockGetVersion.mockResolvedValue({
        message: "New version available: 1.2.3",
        guidelinesHash:
          "02e43fc1ff0ee48db8da468f5c7525877d8056fcd56c77d78a166ac447efb91c",
        agentSkillsSha: sha,
        disableSkillsCli: false,
      });

      await checkVersion(fakeCtx);

      expect(mockGetVersion).toHaveBeenCalled();
      expect(mockLogMessage).toHaveBeenCalledWith(
        "New version available: 1.2.3",
      );
      expect(mockCheckAiFilesStaleness).toHaveBeenCalledWith(
        "02e43fc1ff0ee48db8da468f5c7525877d8056fcd56c77d78a166ac447efb91c",
        sha,
        expect.any(String),
        expect.any(String),
      );
    });

    it("does not log when version has no message", async () => {
      mockGetVersion.mockResolvedValue({
        message: null,
        guidelinesHash:
          "02e43fc1ff0ee48db8da468f5c7525877d8056fcd56c77d78a166ac447efb91c",
        agentSkillsSha: null,
        disableSkillsCli: false,
      });

      await checkVersion(fakeCtx);

      expect(mockGetVersion).toHaveBeenCalled();
      expect(mockLogMessage).not.toHaveBeenCalled();
    });

    it("doesn't do anything when getVersion returns null", async () => {
      mockGetVersion.mockResolvedValue(null);

      await checkVersion(fakeCtx);

      expect(mockGetVersion).toHaveBeenCalled();
      expect(mockLogMessage).not.toHaveBeenCalled();
      expect(mockCheckAiFilesStaleness).not.toHaveBeenCalled();
    });

    it("passes null hashes to staleness check when version has none", async () => {
      mockGetVersion.mockResolvedValue({
        message: "New version available: 1.2.3",
        guidelinesHash: null,
        agentSkillsSha: null,
        disableSkillsCli: false,
      });

      await checkVersion(fakeCtx);

      expect(mockCheckAiFilesStaleness).toHaveBeenCalledWith(
        null,
        null,
        expect.any(String),
        expect.any(String),
      );
    });
  });
});
