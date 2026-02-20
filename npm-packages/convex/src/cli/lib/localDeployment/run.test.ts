import { vi, test, expect, describe, beforeEach, afterEach } from "vitest";
import { oneoffContext } from "../../../bundler/context.js";
import { logFailure } from "../../../bundler/log.js";
import { findLatestVersionWithBinary } from "./download.js";
import { stripVTControlCharacters } from "util";
import { version as npmVersion } from "../../version.js";

async function setupContext() {
  const originalContext = await oneoffContext({
    url: undefined,
    adminKey: undefined,
    envFile: undefined,
  });
  const ctx = {
    ...originalContext,
    crash: (args: { printedMessage: string | null }) => {
      if (args.printedMessage !== null) {
        logFailure(args.printedMessage);
      }
      throw new Error();
    },
  };
  return ctx;
}

describe("findLatestVersionWithBinary", () => {
  beforeEach(() => {
    vi.spyOn(process.stderr, "write").mockImplementation(() => true);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  function stderrOutput(): string {
    const calls = vi.mocked(process.stderr.write).mock.calls;
    return stripVTControlCharacters(String(calls[0][0]));
  }

  test("successfully fetches version from API", async () => {
    const ctx = await setupContext();
    const fetchSpy = vi.spyOn(global, "fetch").mockImplementation(() =>
      Promise.resolve({
        ok: true,
        json: () =>
          Promise.resolve({ version: "precompiled-2026-02-10-4ef979b" }),
      } as Response),
    );

    const version = await findLatestVersionWithBinary(ctx, true);

    expect(version).toBe("precompiled-2026-02-10-4ef979b");
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(fetchSpy).toHaveBeenCalledWith(
      "https://version.convex.dev/v1/local_backend_version",
      {
        headers: { "Convex-Client": `npm-cli-${npmVersion}` },
      },
    );
  });

  test("handles API error with non-200 status", async () => {
    const ctx = await setupContext();
    const fetchSpy = vi.spyOn(global, "fetch").mockImplementation(() =>
      Promise.resolve({
        ok: false,
        status: 500,
        text: () => Promise.resolve("Internal Server Error"),
      } as Response),
    );

    await expect(findLatestVersionWithBinary(ctx, true)).rejects.toThrow();

    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(stderrOutput()).toContain("version.convex.dev returned 500");
  });

  test("handles missing version field in response", async () => {
    const ctx = await setupContext();
    const fetchSpy = vi.spyOn(global, "fetch").mockImplementation(() =>
      Promise.resolve({
        ok: true,
        json: () => Promise.resolve({}),
      } as Response),
    );

    await expect(findLatestVersionWithBinary(ctx, true)).rejects.toThrow();

    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(stderrOutput()).toContain("Invalid response missing version field");
  });

  test("handles network error", async () => {
    const ctx = await setupContext();
    const fetchSpy = vi
      .spyOn(global, "fetch")
      .mockImplementation(() => Promise.reject(new Error("Network error")));

    await expect(findLatestVersionWithBinary(ctx, true)).rejects.toThrow();

    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(stderrOutput()).toContain("Failed to fetch latest backend version");
  });
});
