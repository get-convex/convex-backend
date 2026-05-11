import { describe, it, expect } from "vitest";
import path from "node:path";
import { fileURLToPath } from "node:url";

// These tests pin the contract `buildDepsInner`'s `case "file:":` branch
// depends on: the value handed to `fs.mkdirSync` / `fs.renameSync` must be a
// native filesystem path produced by `fileURLToPath`, not `url.pathname`.
//
// Pre-fix (issue #152): on Windows, `new URL("file:///C:/x/foo.zip").pathname`
// is `/C:/x/foo.zip`, and Windows `fs.mkdirSync` resolves the leading `/`
// against the current drive root, producing `<cwd-drive>:\C:\x\foo.zip` and
// ENOENT.
describe("build_deps file:// path conversion", () => {
  it("fileURLToPath returns a native filesystem path for the current platform", () => {
    const url =
      process.platform === "win32"
        ? new URL("file:///C:/Users/test/.convex/node_modules.zip")
        : new URL("file:///tmp/build_deps/node_modules.zip");

    const filePath = fileURLToPath(url);

    expect(path.isAbsolute(filePath)).toBe(true);
    expect(path.basename(filePath)).toBe("node_modules.zip");
    expect(filePath.startsWith("file:")).toBe(false);
    if (process.platform === "win32") {
      // Drive-letter prefix, no leading slash. `url.pathname` would be `/C:/...`.
      expect(filePath.startsWith("/")).toBe(false);
      expect(/^[A-Za-z]:[\\/]/.test(filePath)).toBe(true);
    }
  });

  it("regression: url.pathname is the broken shape this fix replaces", () => {
    // On every platform, a Windows-shaped `file://` URL produces a `pathname`
    // of the form `/<drive>:/...` — the POSIX-form string that triggered the
    // ENOENT on Windows. Making the broken contract explicit here means the
    // intent of the fix can't quietly drift in a future read.
    const url = new URL("file:///C:/x/y.zip");
    expect(url.pathname).toBe("/C:/x/y.zip");
    expect(url.pathname).toMatch(/^\/[A-Za-z]:\//);
  });
});
