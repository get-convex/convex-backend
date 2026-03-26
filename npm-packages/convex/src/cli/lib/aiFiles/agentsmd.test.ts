import { describe, test, expect, beforeEach, afterEach } from "vitest";
import fs from "fs";
import os from "os";
import path from "path";
import { injectAgentsMdSection, hasAgentsMdInstalled } from "./agentsmd.js";
import {
  AGENTS_MD_START_MARKER,
  AGENTS_MD_END_MARKER,
} from "../../codegen_templates/agentsmd.js";

describe("injectAgentsMdSection", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  const section = `${AGENTS_MD_START_MARKER}\n## Convex\nRead guidelines.\n${AGENTS_MD_END_MARKER}`;

  test("creates AGENTS.md when it does not exist", async () => {
    await injectAgentsMdSection({ section, projectDir: tmpDir });

    const content = fs.readFileSync(path.join(tmpDir, "AGENTS.md"), "utf8");
    expect(content).toContain(AGENTS_MD_START_MARKER);
    expect(content).toContain(AGENTS_MD_END_MARKER);
    expect(content).toContain("## Convex");
  });

  test("appends to an existing AGENTS.md that has no Convex section", async () => {
    const existing = "# My project\n\nSome existing content.\n";
    fs.writeFileSync(path.join(tmpDir, "AGENTS.md"), existing);

    await injectAgentsMdSection({ section, projectDir: tmpDir });

    const content = fs.readFileSync(path.join(tmpDir, "AGENTS.md"), "utf8");
    expect(content).toContain("# My project");
    expect(content).toContain("Some existing content.");
    expect(content).toContain(AGENTS_MD_START_MARKER);
    expect(content).toContain("## Convex");
  });

  test("replaces an existing Convex section when markers are present", async () => {
    const oldSection = `${AGENTS_MD_START_MARKER}\n## Convex\nOld content.\n${AGENTS_MD_END_MARKER}`;
    const existing = `# My project\n\n${oldSection}\n`;
    fs.writeFileSync(path.join(tmpDir, "AGENTS.md"), existing);

    const newSection = `${AGENTS_MD_START_MARKER}\n## Convex\nNew content.\n${AGENTS_MD_END_MARKER}`;
    await injectAgentsMdSection({ section: newSection, projectDir: tmpDir });

    const content = fs.readFileSync(path.join(tmpDir, "AGENTS.md"), "utf8");
    expect(content).toContain("New content.");
    expect(content).not.toContain("Old content.");
    expect(content.split(AGENTS_MD_START_MARKER).length - 1).toBe(1);
  });

  test("preserves content before and after an existing Convex section", async () => {
    const oldSection = `${AGENTS_MD_START_MARKER}\n## Convex\nOld.\n${AGENTS_MD_END_MARKER}`;
    const existing = `# Before\n\n${oldSection}\n\n# After\n`;
    fs.writeFileSync(path.join(tmpDir, "AGENTS.md"), existing);

    await injectAgentsMdSection({ section, projectDir: tmpDir });

    const content = fs.readFileSync(path.join(tmpDir, "AGENTS.md"), "utf8");
    expect(content).toContain("# Before");
    expect(content).toContain("# After");
  });

  test("returns a non-null hash of the written content", async () => {
    const result = await injectAgentsMdSection({ section, projectDir: tmpDir });
    expect(typeof result.sectionHash).toBe("string");
    expect(result.sectionHash.length).toBeGreaterThan(0);
    expect(result.didWrite).toBe(true);
  });

  test("returns hash of the section content, not the entire file", async () => {
    fs.writeFileSync(
      path.join(tmpDir, "AGENTS.md"),
      "# My project\n\nExisting content.\n",
    );

    const result = await injectAgentsMdSection({ section, projectDir: tmpDir });

    const { hashSha256 } = await import("../utils/hash.js");
    expect(result.sectionHash).toBe(hashSha256(section));
  });

  test("does not write when content is unchanged", async () => {
    await injectAgentsMdSection({ section, projectDir: tmpDir });
    const result = await injectAgentsMdSection({ section, projectDir: tmpDir });
    expect(result.didWrite).toBe(false);
  });
});

describe("hasAgentsMdInstalled", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  test("returns false when AGENTS.md does not exist", async () => {
    expect(await hasAgentsMdInstalled(tmpDir)).toBe(false);
  });

  test("returns false when AGENTS.md exists but has no managed markers", async () => {
    fs.writeFileSync(path.join(tmpDir, "AGENTS.md"), "User content only\n");
    expect(await hasAgentsMdInstalled(tmpDir)).toBe(false);
  });

  test("returns false when only the start marker is present", async () => {
    fs.writeFileSync(
      path.join(tmpDir, "AGENTS.md"),
      `${AGENTS_MD_START_MARKER}\npartial content\n`,
    );
    expect(await hasAgentsMdInstalled(tmpDir)).toBe(false);
  });

  test("returns true when both markers are present", async () => {
    fs.writeFileSync(
      path.join(tmpDir, "AGENTS.md"),
      `# Project\n\n${AGENTS_MD_START_MARKER}\n## Convex\n${AGENTS_MD_END_MARKER}\n`,
    );
    expect(await hasAgentsMdInstalled(tmpDir)).toBe(true);
  });
});
