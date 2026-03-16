import { describe, test, expect, beforeEach, afterEach } from "vitest";
import { oneoffContext, Context } from "../../bundler/context.js";
import fs from "fs";
import os from "os";
import path from "path";
import { cleanupStaleGeneratedEntries, doInitConvexFolder } from "./codegen.js";

describe("codegen", () => {
  let tmpDir: string;
  let ctx: Context;

  beforeEach(async () => {
    ctx = await oneoffContext({
      url: undefined,
      adminKey: undefined,
      envFile: undefined,
    });
    tmpDir = fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  test("writes README.md when initializing a new functions directory", async () => {
    const functionsPath = path.join(tmpDir, "convex");
    await doInitConvexFolder(ctx, functionsPath);

    expect(ctx.fs.exists(path.join(functionsPath, "README.md"))).toBe(true);
    expect(ctx.fs.exists(path.join(functionsPath, "tsconfig.json"))).toBe(true);
  });

  test("preserves convex/_generated/ai while deleting stale generated files", () => {
    const codegenDir = path.join(tmpDir, "_generated");
    ctx.fs.mkdir(codegenDir, { recursive: true, allowExisting: true });
    ctx.fs.writeUtf8File(path.join(codegenDir, "server.js"), "export {};");
    ctx.fs.writeUtf8File(path.join(codegenDir, "stale.js"), "export {};");
    const aiDir = path.join(codegenDir, "ai");
    ctx.fs.mkdir(aiDir, { recursive: true, allowExisting: true });
    ctx.fs.writeUtf8File(path.join(aiDir, "ai-files.state.json"), "{}");

    cleanupStaleGeneratedEntries(ctx, codegenDir, ["server.js"]);

    expect(ctx.fs.exists(path.join(codegenDir, "server.js"))).toBe(true);
    expect(ctx.fs.exists(path.join(codegenDir, "stale.js"))).toBe(false);
    expect(ctx.fs.exists(aiDir)).toBe(true);
    expect(ctx.fs.exists(path.join(aiDir, "ai-files.state.json"))).toBe(true);
  });
});
