import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { getLatestLocalBackendVersion } from "./localBackend";
import * as github from "./github";

// Mock the github module
vi.mock("./github", async () => {
  const actual = await vi.importActual("./github");
  return {
    ...actual,
    fetchAllGitHubReleases: vi.fn(),
    findReleaseWithAllAssets: vi.fn(),
  };
});

describe("getLatestLocalBackendVersion", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("should return the tag_name of the first release with all binaries", async () => {
    const mockRelease = {
      tag_name: "precompiled-2025-01-15-abcdef0",
      prerelease: false,
      draft: false,
      assets: [
        { name: "convex-local-backend-aarch64-apple-darwin.zip" },
        { name: "convex-local-backend-x86_64-apple-darwin.zip" },
        { name: "convex-local-backend-aarch64-unknown-linux-gnu.zip" },
        { name: "convex-local-backend-x86_64-unknown-linux-gnu.zip" },
        { name: "convex-local-backend-x86_64-pc-windows-msvc.zip" },
      ],
    };

    vi.mocked(github.fetchAllGitHubReleases).mockResolvedValue([mockRelease]);
    vi.mocked(github.findReleaseWithAllAssets).mockReturnValue(mockRelease);

    const result = await getLatestLocalBackendVersion();

    expect(result).toBe("precompiled-2025-01-15-abcdef0");
    expect(github.fetchAllGitHubReleases).toHaveBeenCalledWith(
      "get-convex/convex-backend",
    );
    expect(github.findReleaseWithAllAssets).toHaveBeenCalledWith(
      [mockRelease],
      [
        "convex-local-backend-aarch64-apple-darwin.zip",
        "convex-local-backend-x86_64-apple-darwin.zip",
        "convex-local-backend-aarch64-unknown-linux-gnu.zip",
        "convex-local-backend-x86_64-unknown-linux-gnu.zip",
        "convex-local-backend-x86_64-pc-windows-msvc.zip",
      ],
    );
  });

  it("should throw when no release has all required binaries", async () => {
    vi.mocked(github.fetchAllGitHubReleases).mockResolvedValue([
      {
        tag_name: "precompiled-2025-01-15-abcdef0",
        prerelease: false,
        draft: false,
        assets: [
          { name: "convex-local-backend-aarch64-apple-darwin.zip" },
          { name: "convex-local-backend-x86_64-apple-darwin.zip" },
        ],
      },
    ]);
    vi.mocked(github.findReleaseWithAllAssets).mockReturnValue(null);

    await expect(getLatestLocalBackendVersion()).rejects.toThrow(
      "Found no stable release with all required local backend binaries.",
    );
  });

  it("should handle GitHub API errors", async () => {
    vi.mocked(github.fetchAllGitHubReleases).mockRejectedValue(
      new Error("GitHub API returned 403: Rate limit exceeded"),
    );

    await expect(getLatestLocalBackendVersion()).rejects.toThrow(
      "GitHub API returned 403: Rate limit exceeded",
    );
  });
});
