import { describe, test, expect, beforeEach, afterEach } from "vitest";
import fs from "fs";
import os from "os";
import path from "path";
import { attemptReadFile } from "./utils.js";

describe("attemptReadFile", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  test("returns content when the file has text", async () => {
    const filePath = path.join(tmpDir, "content.txt");
    fs.writeFileSync(filePath, "hello");

    const result = await attemptReadFile(filePath);

    expect(result).toEqual({ kind: "content", content: "hello" });
  });

  test("returns empty when the file exists but has no contents", async () => {
    const filePath = path.join(tmpDir, "empty.txt");
    fs.writeFileSync(filePath, "");

    const result = await attemptReadFile(filePath);

    expect(result).toEqual({ kind: "empty" });
  });

  test("returns not-found when the file does not exist", async () => {
    const filePath = path.join(tmpDir, "missing.txt");

    const result = await attemptReadFile(filePath);

    expect(result).toEqual({ kind: "not-found" });
  });

  test("throws when fs.readFile fails for another reason", async () => {
    const dirPath = path.join(tmpDir, "nested-dir");
    fs.mkdirSync(dirPath);

    await expect(attemptReadFile(dirPath)).rejects.toThrow();
  });
});
