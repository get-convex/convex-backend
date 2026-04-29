import { describe, it, expect } from "vitest";
import path from "node:path";

import { makeModuleImportUrl } from "./executor";

describe("makeModuleImportUrl", () => {
  it("produces a file:// URL Node's ESM loader accepts", () => {
    const url = makeModuleImportUrl(
      path.join("/tmp", "modules"),
      "actions/auth.js",
      "abc123",
    );
    expect(() => new URL(url)).not.toThrow();
    expect(new URL(url).protocol).toBe("file:");
  });

  it("includes the envHash as a query parameter", () => {
    const url = makeModuleImportUrl("/x", "y.js", "envhash99");
    expect(url).toContain("?envHash=envhash99");
  });

  it("regression: never returns a raw filesystem path", () => {
    // Pre-fix bug (issue #152): on Windows, returning
    // `C:\...\foo.js?envHash=...` made Node's ESM loader throw
    // "Only URLs with a scheme in: file, data, and node are supported.
    //  Received protocol 'c:'". The contract here — applicable on every
    // platform — is that the import argument is always a `file:` URL.
    const url = makeModuleImportUrl("/whatever", "foo.js", "h");
    expect(url.startsWith("file://")).toBe(true);
  });
});
