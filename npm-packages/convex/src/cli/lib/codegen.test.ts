import fs from "fs";
import os from "os";
import path from "path";
import { afterEach, beforeEach, describe, expect, test } from "vitest";
import { Context, oneoffContext } from "../../bundler/context.js";
import { doInitConvexFolder } from "./codegen.js";

describe("doInitConvexFolder", () => {
  let tmpDir: string;
  let functionsDir: string;
  let ctx: Context;

  beforeEach(async () => {
    ctx = await oneoffContext({
      url: undefined,
      adminKey: undefined,
      envFile: undefined,
    });
    tmpDir = fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
    functionsDir = path.join(tmpDir, "convex");
    ctx.fs.mkdir(functionsDir, { recursive: true });
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  test("skipIfExists preserves existing files", async () => {
    const readmePath = path.join(functionsDir, "README.md");
    const tsconfigPath = path.join(functionsDir, "tsconfig.json");
    const readme = "CUSTOM-README-SENTINEL";
    const tsconfig = '{"compilerOptions":{"customSentinel":true}}';

    ctx.fs.writeUtf8File(readmePath, readme);
    ctx.fs.writeUtf8File(tsconfigPath, tsconfig);

    await doInitConvexFolder(ctx, functionsDir, { skipIfExists: true });

    expect(ctx.fs.readUtf8File(readmePath)).toBe(readme);
    expect(ctx.fs.readUtf8File(tsconfigPath)).toBe(tsconfig);
  });

  test("skipIfExists still creates missing files", async () => {
    const readmePath = path.join(functionsDir, "README.md");
    const tsconfigPath = path.join(functionsDir, "tsconfig.json");

    await doInitConvexFolder(ctx, functionsDir, { skipIfExists: true });

    expect(ctx.fs.exists(readmePath)).toBe(true);
    expect(ctx.fs.exists(tsconfigPath)).toBe(true);
  });

  test("default behavior overwrites existing files", async () => {
    const readmePath = path.join(functionsDir, "README.md");
    const tsconfigPath = path.join(functionsDir, "tsconfig.json");
    const readme = "CUSTOM-README-SENTINEL";
    const tsconfig = '{"compilerOptions":{"customSentinel":true}}';

    ctx.fs.writeUtf8File(readmePath, readme);
    ctx.fs.writeUtf8File(tsconfigPath, tsconfig);

    await doInitConvexFolder(ctx, functionsDir);

    expect(ctx.fs.readUtf8File(readmePath)).not.toBe(readme);
    expect(ctx.fs.readUtf8File(tsconfigPath)).not.toBe(tsconfig);
  });
});
