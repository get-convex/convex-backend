import { describe, expect, it } from "vitest";
import { isUseNodeModule } from "./no-import-use-node.js";

describe("isUseNodeModule", () => {
  it.each([
    ['"use node";\nexport {}', "double quotes"],
    ["'use node'\nexport {}", "single quotes, no semicolon"],
    ['// license header\n"use node";\n', "line comment before directive"],
    ['/* block */ "use node";\n', "block comment before directive"],
    ['"use node";\r\nexport {}', "CRLF line endings"],
    ['\uFEFF"use node";\n', "byte order mark"],
    [
      "// " + "x".repeat(150) + '\n"use node";\n',
      "long header before directive",
    ],
  ])("detects the directive: %#", (source) => {
    expect(isUseNodeModule(source)).toBe(true);
  });

  it.each([
    [
      '// No `"use node"` and no SDK: auth.ts imports this file.\nexport function sendCode() {}',
      "comment mentioning the directive (issue #602)",
    ],
    ['/* "use node" */\nexport {}', "block comment mentioning the directive"],
    ['export const s = "use node";', "string in a later statement"],
    ['"use nodejs";', "different directive"],
    ['"use node".length;', "directive text used as an expression"],
    ["export {}", "no directive"],
    ["", "empty file"],
  ])("does not match: %#", (source) => {
    expect(isUseNodeModule(source)).toBe(false);
  });
});
