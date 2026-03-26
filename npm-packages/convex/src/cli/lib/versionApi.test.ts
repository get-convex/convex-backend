import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import {
  getVersion,
  validateVersionResult,
  downloadGuidelines,
  fetchAgentSkillsSha,
} from "./versionApi.js";

// Mock Sentry
vi.mock("@sentry/node", () => ({
  captureException: vi.fn(),
  captureMessage: vi.fn(),
}));

// Mock fetch globally
const mockFetch = vi.fn();
global.fetch = mockFetch;

describe("versionApi", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  describe("getVersion", () => {
    it("returns version data on successful response", async () => {
      const sha = "abc123def456abc123def456abc123def456abc1";
      const mockResponse = {
        ok: true,
        json: vi.fn().mockResolvedValue({
          message: "New version available",
          agentSkillsSha: sha,
        }),
      };
      mockFetch.mockResolvedValue(mockResponse);

      const result = await getVersion();

      expect(result).toEqual({
        kind: "ok",
        data: {
          message: "New version available",
          guidelinesHash: null,
          agentSkillsSha: sha,
          disableSkillsCli: false,
        },
      });
      expect(mockFetch).toHaveBeenCalledWith(
        "https://version.convex.dev/v1/version",
        {
          headers: {
            "Convex-Client": expect.stringMatching(/^npm-cli-/),
            "Convex-Interactive": expect.stringMatching(/^(true|false)$/),
            ...(process.env.CONVEX_AGENT_MODE
              ? { "Convex-Agent-Mode": process.env.CONVEX_AGENT_MODE }
              : {}),
          },
        },
      );
    });

    it("returns error on network error", async () => {
      mockFetch.mockRejectedValue(new Error("Network error"));

      const result = await getVersion();

      expect(result).toEqual({ kind: "error" });
    });

    it("returns error on non-ok response", async () => {
      const mockResponse = {
        ok: false,
        status: 500,
      };
      mockFetch.mockResolvedValue(mockResponse);

      const result = await getVersion();

      expect(result).toEqual({ kind: "error" });
    });

    it("returns error on invalid JSON response", async () => {
      const mockResponse = {
        ok: true,
        json: vi.fn().mockResolvedValue("invalid json"),
      };
      mockFetch.mockResolvedValue(mockResponse);

      const result = await getVersion();

      expect(result).toEqual({ kind: "error" });
    });
  });

  describe("validateVersionResult", () => {
    it("validates correct version result", () => {
      const sha = "abc123def456abc123def456abc123def456abc1";
      const validResult = {
        message: "New version available",
        guidelinesHash: "deadbeef",
        agentSkillsSha: sha,
        disableSkillsCli: true,
      };

      const result = validateVersionResult(validResult);

      expect(result).toEqual(validResult);
    });

    it("validates result with null message", () => {
      const validResult = {
        message: null,
        guidelinesHash: null,
        agentSkillsSha: null,
        disableSkillsCli: false,
      };

      const result = validateVersionResult(validResult);

      expect(result).toEqual(validResult);
    });

    it("treats missing optional hashes as null", () => {
      const result = validateVersionResult({
        message: "New version available",
        // agentSkillsSha and guidelinesHash intentionally absent
      });

      expect(result).toEqual({
        message: "New version available",
        guidelinesHash: null,
        agentSkillsSha: null,
        disableSkillsCli: false,
      });
    });

    it("ignores unknown fields from the server", () => {
      const result = validateVersionResult({
        message: null,
        cursorRulesHash: "legacy-field",
        guidelinesHash: "abc",
        agentSkillsSha: null,
      });

      expect(result).toEqual({
        message: null,
        guidelinesHash: "abc",
        agentSkillsSha: null,
        disableSkillsCli: false,
      });
    });

    it("returns null for non-object input", () => {
      const result = validateVersionResult("not an object");
      expect(result).toBeNull();
    });

    it("returns null for null input", () => {
      const result = validateVersionResult(null);
      expect(result).toBeNull();
    });

    it("returns null for invalid message type", () => {
      const invalidResult = {
        message: 123, // should be string or null
      };

      const result = validateVersionResult(invalidResult);
      expect(result).toBeNull();
    });
  });

  describe("downloadGuidelines", () => {
    it("returns guidelines text on successful response", async () => {
      const mockResponse = {
        ok: true,
        text: vi.fn().mockResolvedValue("# Convex guidelines\n\nUse queries."),
      };
      mockFetch.mockResolvedValue(mockResponse);

      const result = await downloadGuidelines();

      expect(result).toBe("# Convex guidelines\n\nUse queries.");
      expect(mockFetch).toHaveBeenCalledWith(
        "https://version.convex.dev/v1/guidelines",
        {
          headers: {
            "Convex-Client": expect.stringMatching(/^npm-cli-/),
            "Convex-Interactive": expect.stringMatching(/^(true|false)$/),
            ...(process.env.CONVEX_AGENT_MODE
              ? { "Convex-Agent-Mode": process.env.CONVEX_AGENT_MODE }
              : {}),
          },
        },
      );
    });

    it("returns null on network error", async () => {
      mockFetch.mockRejectedValue(new Error("Network error"));

      const result = await downloadGuidelines();

      expect(result).toBeNull();
    });

    it("returns null on non-ok response", async () => {
      mockFetch.mockResolvedValue({ ok: false, status: 500 });

      const result = await downloadGuidelines();

      expect(result).toBeNull();
    });
  });

  describe("fetchAgentSkillsSha", () => {
    it("returns the SHA from version.convex.dev", async () => {
      const sha = "abc123def456abc123def456abc123def456abc1";
      mockFetch.mockResolvedValue({
        ok: true,
        json: vi.fn().mockResolvedValue({
          message: null,
          guidelinesHash: null,
          agentSkillsSha: sha,
          disableSkillsCli: false,
        }),
      });

      const result = await fetchAgentSkillsSha();

      expect(result).toBe(sha);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://version.convex.dev/v1/version",
        {
          headers: {
            "Convex-Client": expect.stringMatching(/^npm-cli-/),
            "Convex-Interactive": expect.stringMatching(/^(true|false)$/),
            ...(process.env.CONVEX_AGENT_MODE
              ? { "Convex-Agent-Mode": process.env.CONVEX_AGENT_MODE }
              : {}),
          },
        },
      );
    });

    it("returns null on network error", async () => {
      mockFetch.mockRejectedValue(new Error("Network error"));

      const result = await fetchAgentSkillsSha();

      expect(result).toBeNull();
    });

    it("returns null on non-ok response", async () => {
      mockFetch.mockResolvedValue({ ok: false, status: 403 });

      const result = await fetchAgentSkillsSha();

      expect(result).toBeNull();
    });

    it("returns null when the version response has no agent skills SHA", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: vi.fn().mockResolvedValue({
          message: null,
          guidelinesHash: null,
          agentSkillsSha: null,
          disableSkillsCli: false,
        }),
      });

      const result = await fetchAgentSkillsSha();

      expect(result).toBeNull();
    });
  });
});
