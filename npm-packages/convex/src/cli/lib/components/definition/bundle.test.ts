import { test, expect, describe } from "vitest";
import { encodeDefinitionPath } from "./bundle.js";
import { ComponentDefinitionPath } from "./directoryStructure.js";

describe("encodeDefinitionPath", async () => {
  test("Escaped definition paths are distinguishable from unescaped ones", () => {
    const a = encodeDefinitionPath("foo/bar-baz" as ComponentDefinitionPath);
    const b = encodeDefinitionPath("foo/bar_baz" as ComponentDefinitionPath);
    expect(a).not.toEqual(b);
  });
});
