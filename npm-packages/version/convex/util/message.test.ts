import { describe, test, expect } from "vitest";
import { generateMessage } from "./message";
import { Doc } from "../_generated/dataModel";

describe("generateMessage", () => {
  const createNpmVersionDoc = (version: string): Doc<"npmVersion"> => ({
    _id: "test" as any,
    _creationTime: Date.now(),
    value: version,
  });

  describe("returns null when no update needed", () => {
    test("returns null when no current version in header", () => {
      const latest = createNpmVersionDoc("1.24.0");
      const result = generateMessage(latest, null);
      expect(result).toBeNull();
    });

    test("returns null when header has no version", () => {
      const latest = createNpmVersionDoc("1.24.0");
      const result = generateMessage(latest, "some-other-header");
      expect(result).toBeNull();
    });

    test("returns null when current version is up to date", () => {
      const latest = createNpmVersionDoc("1.23.0");
      const result = generateMessage(latest, "npm-cli-1.23.0");
      expect(result).toBeNull();
    });

    test("returns null when current version is newer", () => {
      const latest = createNpmVersionDoc("1.23.0");
      const result = generateMessage(latest, "npm-cli-1.24.0");
      expect(result).toBeNull();
    });

    test("handles invalid current version gracefully", () => {
      const latest = createNpmVersionDoc("1.24.0");
      const result = generateMessage(latest, "npm-cli-invalid-version");
      expect(result).toBeNull();
    });

    test("handles invalid latest version gracefully", () => {
      const latest = createNpmVersionDoc("invalid-version");
      const result = generateMessage(latest, "npm-cli-1.23.0");
      expect(result).toBeNull();
    });
  });

  describe("generates update messages", () => {
    test("generates major update message", () => {
      const latest = createNpmVersionDoc("2.0.0");
      const result = generateMessage(latest, "npm-cli-1.23.0");

      expect(result).toContain("A major update is available for Convex");
      expect(result).toContain("(1.23.0 → 2.0.0)");
      expect(result).toContain("Changelog:");
      expect(result).toContain(
        "https://github.com/get-convex/convex-js/blob/main/CHANGELOG.md#changelog",
      );
    });

    test("generates minor update message", () => {
      const latest = createNpmVersionDoc("1.24.0");
      const result = generateMessage(latest, "npm-cli-1.23.0");

      expect(result).toContain("A minor update is available for Convex");
      expect(result).toContain("(1.23.0 → 1.24.0)");
      expect(result).toContain("Changelog:");
    });

    test("generates patch update message", () => {
      const latest = createNpmVersionDoc("1.23.1");
      const result = generateMessage(latest, "npm-cli-1.23.0");

      expect(result).toContain("A patch update is available for Convex");
      expect(result).toContain("(1.23.0 → 1.23.1)");
      expect(result).toContain("Changelog:");
    });

    test("handles complex version numbers", () => {
      const latest = createNpmVersionDoc("1.23.5");
      const result = generateMessage(latest, "npm-cli-1.23.0-alpha.1");

      expect(result).not.toBeNull();
      expect(result).toContain("A patch update is available for Convex");
      expect(result).toContain("(1.23.0-alpha.1 → 1.23.5)");
    });
  });

  describe("message formatting", () => {
    test("message contains proper line breaks", () => {
      const latest = createNpmVersionDoc("1.24.0");
      const result = generateMessage(latest, "npm-cli-1.23.0");

      expect(result).toContain("\n");
      const lines = result!.split("\n");
      expect(lines).toHaveLength(2);
      expect(lines[0]).toContain("A minor update is available");
      expect(lines[1]).toContain("Changelog:");
    });

    test("message contains expected structure", () => {
      const latest = createNpmVersionDoc("1.24.0");
      const result = generateMessage(latest, "npm-cli-1.23.0");

      expect(result).toContain("update is available for Convex");
      expect(result).toContain("Changelog:");
      expect(result).toContain(
        "https://github.com/get-convex/convex-js/blob/main/CHANGELOG.md#changelog",
      );
    });
  });

  describe("edge cases", () => {
    test("handles pre-release current version to stable", () => {
      const latest = createNpmVersionDoc("1.24.0");
      const result = generateMessage(latest, "npm-cli-1.24.0-beta.1");

      expect(result).toContain("A minor update is available for Convex");
      expect(result).toContain("(1.24.0-beta.1 → 1.24.0)");
    });

    test("returns null when pre-release is newer than stable", () => {
      const latest = createNpmVersionDoc("1.23.0");
      const result = generateMessage(latest, "npm-cli-1.24.0-beta.1");

      expect(result).toBeNull();
    });
  });
});
