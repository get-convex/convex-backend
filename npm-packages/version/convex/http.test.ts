import { convexTest } from "convex-test";
import { describe, test, expect, vi, beforeEach } from "vitest";
import schema from "./schema";
import { hashSha256 } from "./util/hash";
import { getLatestCursorRules } from "./util/cursorRules";

vi.mock("./util/cursorRules", () => ({
  getLatestCursorRules: vi.fn(),
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

      // Mock getLatestCursorRules to return new rules
      const mockContent = "new Cursor rules content";
      const mockHash = await hashSha256(mockContent);
      vi.mocked(getLatestCursorRules).mockResolvedValue({
        content: mockContent,
        version: "v1.0.0",
      });

      const response = await t.fetch("/v1/version", {
        method: "GET",
        headers,
      });

      expect(response.status).toBe(200);
      const data = await response.json();
      // Message should be null when version is current
      expect(data).toHaveProperty("message", null);
      expect(data).toHaveProperty("cursorRulesHash", mockHash);
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

      // Mock getLatestCursorRules to return new rules
      const mockContent = "api cursor rules content";
      const mockHash = await hashSha256(mockContent);
      vi.mocked(getLatestCursorRules).mockResolvedValue({
        content: mockContent,
        version: "v1.0.0",
      });

      const response = await t.fetch("/v1/version", {
        method: "GET",
        headers,
      });

      expect(response.status).toBe(200);
      const data = await response.json();
      // Message should be null when no version is provided
      expect(data).toHaveProperty("message", null);
      expect(data).toHaveProperty("cursorRulesHash", mockHash);
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
});

const modules = import.meta.glob("./**/*.ts");
