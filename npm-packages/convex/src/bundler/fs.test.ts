/* eslint-disable no-restricted-syntax */
import { test, expect } from "@jest/globals";
import fs from "fs";
import os from "os";
import path from "path";
import { nodeFs, RecordingFs } from "./fs.js";

test("nodeFs filesystem operations behave as expected", async () => {
  const tmpdir = fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
  try {
    const parentDirPath = path.join(tmpdir, "testdir");
    const dirPath = path.join(parentDirPath, "nestedDir");
    const filePath = path.join(dirPath, "text.txt");

    // Test making and listing directories.
    try {
      nodeFs.mkdir(dirPath);
      throw new Error(
        "Expected `mkdir` to fail because the containing directory doesn't exist yet.",
      );
    } catch (e: any) {
      expect(e.code).toEqual("ENOENT");
    }

    nodeFs.mkdir(parentDirPath);
    nodeFs.mkdir(dirPath);
    try {
      nodeFs.mkdir(dirPath);
      throw new Error("Expected `mkdir` to fail without allowExisting");
    } catch (e: any) {
      expect(e.code).toEqual("EEXIST");
    }
    nodeFs.mkdir(dirPath, { allowExisting: true });

    const dirEntries = nodeFs.listDir(parentDirPath);
    expect(dirEntries).toHaveLength(1);
    expect(dirEntries[0].name).toEqual("nestedDir");

    const nestedEntries = nodeFs.listDir(dirPath);
    expect(nestedEntries).toHaveLength(0);

    // Test file based methods for nonexistent paths.
    expect(nodeFs.exists(filePath)).toEqual(false);
    try {
      nodeFs.stat(filePath);
      throw new Error("Expected `stat` to fail for nonexistent paths");
    } catch (e: any) {
      expect(e.code).toEqual("ENOENT");
    }
    try {
      nodeFs.readUtf8File(filePath);
      throw new Error("Expected `readUtf8File` to fail for nonexistent paths");
    } catch (e: any) {
      expect(e.code).toEqual("ENOENT");
    }
    try {
      nodeFs.access(filePath);
      throw new Error("Expected `access` to fail for nonexistent paths");
    } catch (e: any) {
      expect(e.code).toEqual("ENOENT");
    }

    // Test creating a file and accessing it.
    const message = "it's trompo o'clock";
    nodeFs.writeUtf8File(filePath, message);
    expect(nodeFs.exists(filePath)).toEqual(true);
    nodeFs.stat(filePath);
    expect(nodeFs.readUtf8File(filePath)).toEqual(message);
    nodeFs.access(filePath);

    // Test unlinking a file and directory.
    try {
      nodeFs.unlink(dirPath);
      throw new Error("Expected `unlink` to fail on a directory");
    } catch (e: any) {
      if (os.platform() === "linux") {
        expect(e.code).toEqual("EISDIR");
      } else {
        expect(e.code).toEqual("EPERM");
      }
    }
    nodeFs.unlink(filePath);
    expect(nodeFs.exists(filePath)).toEqual(false);
  } finally {
    fs.rmSync(tmpdir, { recursive: true });
  }
});

describe("RecordingFs", () => {
  let tmpDir: string;
  let recordingFs: RecordingFs;
  beforeEach(() => {
    tmpDir = fs.mkdtempSync(`${os.tmpdir()}${path.sep}`);
    recordingFs = new RecordingFs(false);
  });

  describe("rm", () => {
    test("deletes file", () => {
      const file = path.join(tmpDir, "file");
      recordingFs.writeUtf8File(file, "contents");
      expect(recordingFs.exists(file)).toBe(true);

      recordingFs.rm(file);
      expect(recordingFs.exists(file)).toBe(false);
    });

    test("throws an error on non-existent file", () => {
      const nonexistentFile = path.join(tmpDir, "nonexistent_file");
      expect(() => {
        recordingFs.rm(nonexistentFile);
      }).toThrow("ENOENT: no such file or directory");
    });

    test("does not throw error if `force` is used", () => {
      const nonexistentFile = path.join(tmpDir, "nonexistent_file");
      recordingFs.rm(nonexistentFile, { force: true });
    });

    test("recursively deletes a directory", () => {
      const dir = path.join(tmpDir, "dir");
      recordingFs.mkdir(dir);
      const nestedFile = path.join(dir, "nested_file");
      recordingFs.writeUtf8File(nestedFile, "content");
      const nestedDir = path.join(dir, "nested_dir");
      recordingFs.mkdir(nestedDir);

      expect(recordingFs.exists(dir)).toBe(true);

      recordingFs.rm(dir, { recursive: true });
      expect(recordingFs.exists(dir)).toBe(false);
    });

    test("`recursive` and `force` work together", () => {
      const nonexistentDir = path.join(tmpDir, "nonexistent_dir");
      // Shouldn't throw an exception.
      recordingFs.rm(nonexistentDir, { force: true, recursive: true });
    });
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true });
  });
});
