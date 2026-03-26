import { describe, test, expect, beforeEach, afterEach } from "vitest";
import fs from "fs";
import os from "os";
import path from "path";
import { injectClaudeMdSection, hasClaudeMdInstalled } from "./claudemd.js";
import {
  CLAUDE_MD_START_MARKER,
  CLAUDE_MD_END_MARKER,
} from "../../codegen_templates/claudemd.js";

describe("injectClaudeMdSection", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  const section = `${CLAUDE_MD_START_MARKER}\n## Convex\nRead guidelines.\n${CLAUDE_MD_END_MARKER}`;

  test("creates CLAUDE.md when it does not exist", async () => {
    const result = await injectClaudeMdSection({ section, projectDir: tmpDir });

    const content = fs.readFileSync(path.join(tmpDir, "CLAUDE.md"), "utf8");
    expect(content).toContain(CLAUDE_MD_START_MARKER);
    expect(content).toContain(CLAUDE_MD_END_MARKER);
    expect(result.didWrite).toBe(true);
  });

  test("appends managed section to existing CLAUDE.md content", async () => {
    fs.writeFileSync(
      path.join(tmpDir, "CLAUDE.md"),
      "My custom instructions\n",
    );

    const result = await injectClaudeMdSection({ section, projectDir: tmpDir });

    const content = fs.readFileSync(path.join(tmpDir, "CLAUDE.md"), "utf8");
    expect(content).toContain("My custom instructions");
    expect(content).toContain(CLAUDE_MD_START_MARKER);
    expect(result.didWrite).toBe(true);
  });

  test("replaces managed section without touching user content", async () => {
    const oldSection = `${CLAUDE_MD_START_MARKER}\nOld\n${CLAUDE_MD_END_MARKER}`;
    fs.writeFileSync(
      path.join(tmpDir, "CLAUDE.md"),
      `# Header\n\n${oldSection}\n\n# Footer\n`,
      "utf8",
    );

    await injectClaudeMdSection({ section, projectDir: tmpDir });

    const content = fs.readFileSync(path.join(tmpDir, "CLAUDE.md"), "utf8");
    expect(content).toContain("# Header");
    expect(content).toContain("# Footer");
    expect(content).toContain("## Convex");
    expect(content).not.toContain("Old");
  });
});

describe("hasClaudeMdInstalled", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  test("returns false when CLAUDE.md does not exist", async () => {
    expect(await hasClaudeMdInstalled(tmpDir)).toBe(false);
  });

  test("returns false when CLAUDE.md exists but has no managed markers", async () => {
    fs.writeFileSync(path.join(tmpDir, "CLAUDE.md"), "User content only\n");
    expect(await hasClaudeMdInstalled(tmpDir)).toBe(false);
  });

  test("returns true when both markers are present", async () => {
    fs.writeFileSync(
      path.join(tmpDir, "CLAUDE.md"),
      `${CLAUDE_MD_START_MARKER}\n## Convex\n${CLAUDE_MD_END_MARKER}\n`,
    );
    expect(await hasClaudeMdInstalled(tmpDir)).toBe(true);
  });
});
