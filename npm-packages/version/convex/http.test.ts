import { convexTest } from "convex-test";
import { describe, test, expect, vi, beforeEach } from "vitest";
import schema from "./schema";
import { hashSha256 } from "./util/hash";
import { getLatestCursorRules } from "./util/cursorRules";
import { getLatestGuidelines } from "./util/guidelines";
import { getLatestAgentSkillsSha } from "./util/agentSkills";

vi.mock("./util/cursorRules", () => ({
  getLatestCursorRules: vi.fn(),
}));

vi.mock("./util/guidelines", () => ({
  getLatestGuidelines: vi.fn(),
}));

vi.mock("./util/agentSkills", () => ({
  getLatestAgentSkillsSha: vi.fn(),
}));

vi.mock("./util/npm", () => ({
  fetchLatestNpmVersion: vi.fn().mockResolvedValue("1.31.2"),
}));

describe("HTTP endpoints", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("`convex` version after explicit table IDs", () => {
    const headers = {
      "Convex-Client": "npm-cli-1.31.2",
    };

    test("GET /v1/version returns version info with new Cursor rules", async () => {
      const t = convexTest(schema, modules);

      const mockContent = "new Cursor rules content";
      const mockHash = await hashSha256(mockContent);
      vi.mocked(getLatestCursorRules).mockResolvedValue({
        content: mockContent,
        version: "v1.0.0",
      });

      const mockGuidelinesContent = "new guidelines content";
      const mockGuidelinesHash = await hashSha256(mockGuidelinesContent);
      vi.mocked(getLatestGuidelines).mockResolvedValue({
        content: mockGuidelinesContent,
        version: "v1.0.0",
      });

      const mockSkillsSha = "abc123def456abc123def456abc123def456abc1";
      vi.mocked(getLatestAgentSkillsSha).mockResolvedValue(mockSkillsSha);

      const response = await t.fetch("/v1/version", {
        method: "GET",
        headers,
      });

      expect(response.status).toBe(200);
      const data = await response.json();
      expect(data).toHaveProperty("message", null);
      expect(data).toHaveProperty("cursorRulesHash", mockHash);
      expect(data).toHaveProperty("guidelinesHash", mockGuidelinesHash);
      expect(data).toHaveProperty("agentSkillsSha", mockSkillsSha);
    });

    test("GET /v1/cursor_rules returns new cursor rules", async () => {
      const t = convexTest(schema, modules);

      // Mock getLatestCursorRules to return new rules
      const mockContent = "new Cursor rules content";
      vi.mocked(getLatestCursorRules).mockResolvedValue({
        content: mockContent,
        version: "v1.0.0",
      });

      const response = await t.fetch("/v1/cursor_rules", {
        method: "GET",
        headers,
      });

      expect(response.status).toBe(200);
      const content = await response.text();
      expect(content).toBe(mockContent);
    });
  });

  describe("`convex` version before explicit table IDs", () => {
    const headers = {
      "Convex-Client": "npm-cli-1.29.0",
    };

    test("GET /v1/version returns version info with old cursor rules", async () => {
      const t = convexTest(schema, modules);

      const mockGuidelinesContent = "new guidelines content";
      vi.mocked(getLatestGuidelines).mockResolvedValue({
        content: mockGuidelinesContent,
        version: "v1.0.0",
      });

      const response = await t.fetch("/v1/version", {
        method: "GET",
        headers,
      });

      expect(response.status).toBe(200);
      const data = await response.json();
      // Message should contain something when version is old
      expect(data).toHaveProperty("message");
      expect(data.message).not.toBeNull();
      expect(data.message).toContain("update is available");
      // For old versions, should use OLD_CURSOR_RULES
      expect(data).toHaveProperty("cursorRulesHash");
      expect(data.cursorRulesHash).not.toBeNull();
    });

    test("GET /v1/cursor_rules returns old Cursor rules", async () => {
      const t = convexTest(schema, modules);

      const response = await t.fetch("/v1/cursor_rules", {
        method: "GET",
        headers,
      });

      expect(response.status).toBe(200);
      const content = await response.text();
      // Should return old Cursor rules content
      expect(content).toContain("Convex guidelines");
    });
  });

  describe("no `Convex-Client` header", () => {
    const headers = {};

    test("GET /v1/version returns version info with API Cursor rules", async () => {
      const t = convexTest(schema, modules);

      const mockContent = "api cursor rules content";
      const mockHash = await hashSha256(mockContent);
      vi.mocked(getLatestCursorRules).mockResolvedValue({
        content: mockContent,
        version: "v1.0.0",
      });

      const mockGuidelinesContent = "api guidelines content";
      const mockGuidelinesHash = await hashSha256(mockGuidelinesContent);
      vi.mocked(getLatestGuidelines).mockResolvedValue({
        content: mockGuidelinesContent,
        version: "v1.0.0",
      });

      const mockSkillsSha = "abc123def456abc123def456abc123def456abc1";
      vi.mocked(getLatestAgentSkillsSha).mockResolvedValue(mockSkillsSha);

      const response = await t.fetch("/v1/version", {
        method: "GET",
        headers,
      });

      expect(response.status).toBe(200);
      const data = await response.json();
      expect(data).toHaveProperty("message", null);
      expect(data).toHaveProperty("cursorRulesHash", mockHash);
      expect(data).toHaveProperty("guidelinesHash", mockGuidelinesHash);
      expect(data).toHaveProperty("agentSkillsSha", mockSkillsSha);
    });

    test("GET /v1/cursor_rules returns API Cursor rules", async () => {
      const t = convexTest(schema, modules);

      // Mock getLatestCursorRules to return new rules
      const mockContent = "api cursor rules content";
      vi.mocked(getLatestCursorRules).mockResolvedValue({
        content: mockContent,
        version: "v1.0.0",
      });

      const response = await t.fetch("/v1/cursor_rules", {
        method: "GET",
        headers,
      });

      expect(response.status).toBe(200);
      const content = await response.text();
      expect(content).toBe(mockContent);
    });
  });

  describe("/v1/guidelines", () => {
    test("GET /v1/guidelines returns latest guidelines content", async () => {
      const t = convexTest(schema, modules);

      const mockGuidelinesContent = "guidelines endpoint content";
      vi.mocked(getLatestGuidelines).mockResolvedValue({
        content: mockGuidelinesContent,
        version: "v1.0.0",
      });

      const response = await t.fetch("/v1/guidelines", { method: "GET" });

      expect(response.status).toBe(200);
      expect(await response.text()).toBe(mockGuidelinesContent);
      expect(response.headers.get("content-type")).toContain("text/plain");
    });

    test("GET /v1/guidelines returns 500 when refresh fails", async () => {
      const t = convexTest(schema, modules);
      vi.mocked(getLatestGuidelines).mockRejectedValue(
        new Error("simulated fetch failure"),
      );

      const response = await t.fetch("/v1/guidelines", { method: "GET" });

      expect(response.status).toBe(500);
      expect(await response.text()).toContain("Can't get guidelines");
    });
  });
});

const modules = import.meta.glob("./**/*.ts");
