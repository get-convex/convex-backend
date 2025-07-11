import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  parseLinkHeader,
  fetchAllGitHubReleases,
  findReleaseWithAsset,
  downloadAssetFromRelease,
} from "./github";

// Mock fetch globally
const mockFetch = vi.fn();
global.fetch = mockFetch;

describe("GitHub Helper Functions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("parseLinkHeader", () => {
    it("should parse a complete Link header with all relations", () => {
      const header =
        '<https://api.github.com/repos/test/repo/releases?page=2>; rel="prev", <https://api.github.com/repos/test/repo/releases?page=4>; rel="next", <https://api.github.com/repos/test/repo/releases?page=10>; rel="last", <https://api.github.com/repos/test/repo/releases?page=1>; rel="first"';

      const result = parseLinkHeader(header);

      expect(result).toEqual({
        prev: "https://api.github.com/repos/test/repo/releases?page=2",
        next: "https://api.github.com/repos/test/repo/releases?page=4",
        last: "https://api.github.com/repos/test/repo/releases?page=10",
        first: "https://api.github.com/repos/test/repo/releases?page=1",
      });
    });

    it("should parse a partial Link header with only next relation", () => {
      const header =
        '<https://api.github.com/repos/test/repo/releases?page=2>; rel="next"';

      const result = parseLinkHeader(header);

      expect(result).toEqual({
        next: "https://api.github.com/repos/test/repo/releases?page=2",
      });
    });

    it("should handle empty header", () => {
      const result = parseLinkHeader("");
      expect(result).toEqual({});
    });

    it("should handle malformed header sections", () => {
      const header =
        '<https://api.github.com/repos/test/repo/releases?page=2>, <https://api.github.com/repos/test/repo/releases?page=3>; rel="next"';

      const result = parseLinkHeader(header);

      expect(result).toEqual({
        next: "https://api.github.com/repos/test/repo/releases?page=3",
      });
    });
  });

  describe("fetchAllGitHubReleases", () => {
    const mockReleases1 = [
      { tag_name: "v1.0.0", prerelease: false, draft: false, assets: [] },
      { tag_name: "v0.9.0", prerelease: false, draft: false, assets: [] },
    ];

    const mockReleases2 = [
      { tag_name: "v0.8.0", prerelease: false, draft: false, assets: [] },
    ];

    it("should fetch all releases with pagination", async () => {
      // First page
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => mockReleases1,
        headers: {
          get: (name: string) =>
            name === "Link"
              ? '<https://api.github.com/repos/test/repo/releases?page=2>; rel="next"'
              : null,
        },
      });

      // Second page
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => mockReleases2,
        headers: {
          get: () => null, // No Link header, end of pagination
        },
      });

      const result = await fetchAllGitHubReleases("test/repo");

      expect(result).toEqual([...mockReleases1, ...mockReleases2]);
      expect(mockFetch).toHaveBeenCalledTimes(2);
      expect(mockFetch).toHaveBeenNthCalledWith(
        1,
        "https://api.github.com/repos/test/repo/releases?per_page=30",
      );
      expect(mockFetch).toHaveBeenNthCalledWith(
        2,
        "https://api.github.com/repos/test/repo/releases?page=2",
      );
    });

    it("should handle single page of releases", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => mockReleases1,
        headers: {
          get: () => null, // No Link header
        },
      });

      const result = await fetchAllGitHubReleases("test/repo");

      expect(result).toEqual(mockReleases1);
      expect(mockFetch).toHaveBeenCalledTimes(1);
    });

    it("should handle empty releases array", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => [],
        headers: {
          get: () => null,
        },
      });

      const result = await fetchAllGitHubReleases("test/repo");

      expect(result).toEqual([]);
      expect(mockFetch).toHaveBeenCalledTimes(1);
    });

    it("should throw error on API failure", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 404,
        text: async () => "Not Found",
      });

      await expect(fetchAllGitHubReleases("test/repo")).rejects.toThrow(
        "GitHub API returned 404: Not Found",
      );
    });

    it("should use custom perPage parameter", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => [],
        headers: {
          get: () => null,
        },
      });

      await fetchAllGitHubReleases("test/repo", 50);

      expect(mockFetch).toHaveBeenCalledWith(
        "https://api.github.com/repos/test/repo/releases?per_page=50",
      );
    });
  });

  describe("findReleaseWithAsset", () => {
    const mockReleases = [
      {
        tag_name: "v1.0.0",
        prerelease: false,
        draft: false,
        assets: [{ name: "app.zip" }, { name: "docs.pdf" }],
      },
      {
        tag_name: "v0.9.0-beta",
        prerelease: true,
        draft: false,
        assets: [{ name: "convex_rules.mdc" }],
      },
      {
        tag_name: "v0.8.0",
        prerelease: false,
        draft: false,
        assets: [{ name: "convex_rules.mdc" }, { name: "other.txt" }],
      },
      {
        tag_name: "v0.7.0",
        prerelease: false,
        draft: true,
        assets: [{ name: "convex_rules.mdc" }],
      },
    ];

    it("should find the first stable release with the asset", () => {
      const result = findReleaseWithAsset(mockReleases, "convex_rules.mdc");

      expect(result).toEqual(mockReleases[2]); // v0.8.0
    });

    it("should return null if no stable release has the asset", () => {
      const result = findReleaseWithAsset(mockReleases, "nonexistent.file");

      expect(result).toBeNull();
    });

    it("should skip prerelease versions", () => {
      const result = findReleaseWithAsset(mockReleases, "convex_rules.mdc");

      expect(result?.tag_name).toBe("v0.8.0"); // Should skip v0.9.0-beta
    });

    it("should skip draft versions", () => {
      const releases = [
        {
          tag_name: "v1.0.0",
          prerelease: false,
          draft: true,
          assets: [{ name: "convex_rules.mdc" }],
        },
        {
          tag_name: "v0.9.0",
          prerelease: false,
          draft: false,
          assets: [{ name: "convex_rules.mdc" }],
        },
      ];

      const result = findReleaseWithAsset(releases, "convex_rules.mdc");

      expect(result?.tag_name).toBe("v0.9.0");
    });

    it("should log verbose information when verbose is true", () => {
      const consoleSpy = vi.spyOn(console, "log").mockImplementation(() => {});

      findReleaseWithAsset(mockReleases, "convex_rules.mdc", { verbose: true });

      expect(consoleSpy).toHaveBeenCalledWith(
        "Latest stable version with convex_rules.mdc is v0.8.0",
      );

      consoleSpy.mockRestore();
    });

    it("should log when asset is not found in verbose mode", () => {
      const consoleSpy = vi.spyOn(console, "log").mockImplementation(() => {});

      findReleaseWithAsset(mockReleases, "nonexistent.file", { verbose: true });

      expect(consoleSpy).toHaveBeenCalledWith(
        "Version v1.0.0 does not contain a nonexistent.file, checking previous version",
      );
      expect(consoleSpy).toHaveBeenCalledWith("ASSETS: app.zip, docs.pdf");

      consoleSpy.mockRestore();
    });
  });

  describe("downloadAssetFromRelease", () => {
    it("should successfully download an asset", async () => {
      const mockContent = "Mock file content";

      mockFetch.mockResolvedValueOnce({
        ok: true,
        text: async () => mockContent,
      });

      const result = await downloadAssetFromRelease(
        "test/repo",
        "v1.0.0",
        "convex_rules.mdc",
      );

      expect(result).toBe(mockContent);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://github.com/test/repo/releases/download/v1.0.0/convex_rules.mdc",
      );
    });

    it("should throw error on download failure", async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 404,
      });

      await expect(
        downloadAssetFromRelease("test/repo", "v1.0.0", "convex_rules.mdc"),
      ).rejects.toThrow(
        "Failed to download convex_rules.mdc from https://github.com/test/repo/releases/download/v1.0.0/convex_rules.mdc",
      );
    });

    it("should handle network errors", async () => {
      mockFetch.mockRejectedValueOnce(new Error("Network error"));

      await expect(
        downloadAssetFromRelease("test/repo", "v1.0.0", "convex_rules.mdc"),
      ).rejects.toThrow("Network error");
    });
  });
});
