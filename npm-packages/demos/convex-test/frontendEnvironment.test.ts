import { describe, expect, it } from "vitest";

describe("frontend test environment", () => {
  it("exposes jsdom globals", () => {
    const globals = globalThis as typeof globalThis & {
      document?: { defaultView?: unknown };
      window?: unknown;
    };

    expect(globals.document).toBeDefined();
    expect(globals.window).toBeDefined();
    expect(globals.document?.defaultView).toBe(globals.window);
  });
});
