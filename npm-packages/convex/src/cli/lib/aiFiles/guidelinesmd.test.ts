import { describe, test, expect, beforeEach, afterEach } from "vitest";
import fs from "fs";
import os from "os";
import path from "path";
import { hasGuidelinesInstalled } from "./guidelinesmd.js";

describe("hasGuidelinesInstalled", () => {
  let tmpDir: string;
  let convexDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
    convexDir = path.join(tmpDir, "convex");
    fs.mkdirSync(path.join(convexDir, "_generated", "ai"), { recursive: true });
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  test("returns false when guidelines.md does not exist", async () => {
    expect(await hasGuidelinesInstalled(convexDir)).toBe(false);
  });

  test("returns true when guidelines.md exists", async () => {
    fs.writeFileSync(
      path.join(convexDir, "_generated", "ai", "guidelines.md"),
      "some content",
    );
    expect(await hasGuidelinesInstalled(convexDir)).toBe(true);
  });

  test("returns true even when guidelines.md is empty", async () => {
    fs.writeFileSync(
      path.join(convexDir, "_generated", "ai", "guidelines.md"),
      "",
    );
    expect(await hasGuidelinesInstalled(convexDir)).toBe(true);
  });
});
