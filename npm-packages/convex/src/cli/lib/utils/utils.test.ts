import { describe, test, expect, vi, afterEach, beforeEach } from "vitest";
// Must be imported before any module that uses @inquirer/*
import { screen } from "@inquirer/testing/vitest";
import type { BigBrainAuth, Context } from "../../../bundler/context.js";
import {
  bigBrainFetch,
  bigBrainAPIMaybeThrows,
  BIG_BRAIN_URL,
  selectRegion,
} from "./utils.js";

// Hoist this mock so it's in place before `retryingFetch` is initialized.
// We make the factory return a function that delegates to `globalThis.fetch`
// at call time, so tests can stub it via `vi.stubGlobal`.
// vitest hoists vi.mock() calls before imports, so this applies to the
// `retryingFetch = fetchRetryFactory(fetch)` initialization in utils.ts.
vi.mock("fetch-retry", () => ({
  default: (_fetch: any) => (resource: any, options: any) =>
    (globalThis as any).fetch(resource, options),
}));

function makeContext(auth: BigBrainAuth | null): Context {
  return {
    bigBrainAuth: () => auth,
    crash: vi.fn(),
    registerCleanup: vi.fn(),
    removeCleanup: vi.fn(),
    _updateBigBrainAuth: vi.fn(),
    fs: {} as any,
    deprecationMessagePrinted: false,
    spinner: undefined,
  } as unknown as Context;
}

function stubFetch() {
  const mockFetch = vi
    .fn()
    .mockResolvedValue(new Response("{}", { status: 200 }));
  vi.stubGlobal("fetch", mockFetch);
  return mockFetch;
}

function capturedArgs(mockFetch: ReturnType<typeof vi.fn>) {
  const [resource, options] = mockFetch.mock.calls[0] as [
    RequestInfo | URL,
    RequestInit & any,
  ];
  return {
    resource,
    options,
    headers: options.headers as Record<string, string>,
  };
}

function capturedOptionsHeaders(mockFetch: ReturnType<typeof vi.fn>) {
  return capturedArgs(mockFetch).headers;
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("bigBrainFetch", () => {
  test("sets Convex-Client header and no Authorization when auth is null", async () => {
    const mockFetch = stubFetch();
    const ctx = makeContext(null);

    const fetch = bigBrainFetch(ctx);
    await fetch("https://api.convex.dev/api/test", { method: "GET" });

    expect(mockFetch).toHaveBeenCalledOnce();
    const headers = capturedOptionsHeaders(mockFetch);
    expect(headers["Convex-Client"]).toMatch(/^npm-cli-/);
    expect(headers["Authorization"]).toBeUndefined();
  });

  test("sets both Authorization and Convex-Client headers when auth is present", async () => {
    const mockFetch = stubFetch();
    const ctx = makeContext({
      kind: "accessToken",
      header: "Bearer test-token",
      accessToken: "test-token",
    });

    const fetch = bigBrainFetch(ctx);
    await fetch("https://api.convex.dev/api/test", { method: "GET" });

    expect(mockFetch).toHaveBeenCalledOnce();
    const headers = capturedOptionsHeaders(mockFetch);
    expect(headers["Authorization"]).toBe("Bearer test-token");
    expect(headers["Convex-Client"]).toMatch(/^npm-cli-/);
  });

  test("caller-supplied options headers are merged over bigBrain headers", async () => {
    const mockFetch = stubFetch();
    const ctx = makeContext({
      kind: "accessToken",
      header: "Bearer original-token",
      accessToken: "original-token",
    });

    const fetch = bigBrainFetch(ctx);
    await fetch("https://api.convex.dev/api/test", {
      method: "POST",
      headers: {
        Authorization: "Bearer caller-override",
        "Content-Type": "application/json",
      },
    });

    expect(mockFetch).toHaveBeenCalledOnce();
    const headers = capturedOptionsHeaders(mockFetch);
    expect(headers["Authorization"]).toBe("Bearer caller-override");
    expect(headers["Content-Type"]).toBe("application/json");
    expect(headers["Convex-Client"]).toMatch(/^npm-cli-/);
  });

  test("uses Request object headers when no options headers provided", async () => {
    const mockFetch = stubFetch();
    const ctx = makeContext(null);

    const request = new Request("https://api.convex.dev/api/test", {
      method: "GET",
      headers: { "X-Custom-Header": "custom-value" },
    });

    const fetch = bigBrainFetch(ctx);
    await fetch(request, undefined);

    expect(mockFetch).toHaveBeenCalledOnce();
    const headers = capturedOptionsHeaders(mockFetch);
    // The Headers object normalizes header names to lowercase when iterated.
    expect(headers["x-custom-header"]).toBe("custom-value");
    expect(headers["Convex-Client"]).toMatch(/^npm-cli-/);
  });

  test("options headers take precedence over Request object headers", async () => {
    const mockFetch = stubFetch();
    const ctx = makeContext(null);

    const request = new Request("https://api.convex.dev/api/test", {
      method: "GET",
      headers: { "X-Custom-Header": "from-request" },
    });

    const fetch = bigBrainFetch(ctx);
    await fetch(request, {
      headers: { "X-Custom-Header": "from-options" },
    });

    expect(mockFetch).toHaveBeenCalledOnce();
    const headers = capturedOptionsHeaders(mockFetch);
    expect(headers["X-Custom-Header"]).toBe("from-options");
  });

  test("passes the request through to throwingFetch", async () => {
    const mockFetch = stubFetch();
    const ctx = makeContext(null);
    const request = new Request("https://api.convex.dev/api/some-endpoint", {
      method: "POST",
    });

    const fetch = bigBrainFetch(ctx);
    await fetch(request);

    expect(mockFetch).toHaveBeenCalledOnce();
    const [resource] = mockFetch.mock.calls[0] as [any, any];
    expect(resource).toBe(request);
  });
});

describe("bigBrainAPIMaybeThrows", () => {
  test("joins url with BIG_BRAIN_URL", async () => {
    const mockFetch = stubFetch();
    const ctx = makeContext(null);

    await bigBrainAPIMaybeThrows({ ctx, method: "GET", path: "has_projects" });

    expect(mockFetch).toHaveBeenCalledOnce();
    const { resource } = capturedArgs(mockFetch);
    expect(resource.toString()).toBe(
      new URL("has_projects", BIG_BRAIN_URL).toString(),
    );
  });

  test("passes method through for GET with no body", async () => {
    const mockFetch = stubFetch();
    const ctx = makeContext(null);

    await bigBrainAPIMaybeThrows({ ctx, method: "GET", path: "test" });

    expect(mockFetch).toHaveBeenCalledOnce();
    const { options } = capturedArgs(mockFetch);
    expect(options.method).toBe("GET");
    expect(options.body).toBeUndefined();
  });

  test("POST with no data sends empty object body", async () => {
    const mockFetch = stubFetch();
    const ctx = makeContext(null);

    await bigBrainAPIMaybeThrows({ ctx, method: "POST", path: "test" });

    expect(mockFetch).toHaveBeenCalledOnce();
    const { options, headers } = capturedArgs(mockFetch);
    expect(options.method).toBe("POST");
    expect(options.body).toBe("{}");
    expect(headers["Content-Type"]).toBe("application/json");
  });

  test("POST with object data serializes it to JSON", async () => {
    const mockFetch = stubFetch();
    const ctx = makeContext(null);
    const data = { name: "test", value: 42 };

    await bigBrainAPIMaybeThrows({ ctx, method: "POST", path: "test", data });

    expect(mockFetch).toHaveBeenCalledOnce();
    const { options } = capturedArgs(mockFetch);
    expect(options.body).toBe(JSON.stringify(data));
  });

  test("POST with string data passes it through as-is", async () => {
    const mockFetch = stubFetch();
    const ctx = makeContext(null);

    await bigBrainAPIMaybeThrows({
      ctx,
      method: "POST",
      path: "test",
      data: '{"raw":true}',
    });

    expect(mockFetch).toHaveBeenCalledOnce();
    const { options } = capturedArgs(mockFetch);
    expect(options.body).toBe('{"raw":true}');
  });
});

const testRegions = [
  { name: "aws-eu-west-1", displayName: "EU West (Ireland)", available: true },
  {
    name: "aws-us-east-1",
    displayName: "US East (Virginia)",
    available: true,
  },
  {
    name: "aws-ap-southeast-1",
    displayName: "Asia Pacific (Singapore)",
    available: false,
  },
];

function stubRegionsFetch() {
  const mockFetch = vi.fn().mockImplementation(async (resource: any) => {
    const url = resource instanceof Request ? resource.url : String(resource);
    if (url.includes("/list_deployment_regions")) {
      return new Response(JSON.stringify({ items: testRegions }), {
        status: 200,
      });
    }
    return new Response("{}", { status: 200 });
  });
  vi.stubGlobal("fetch", mockFetch);
  return mockFetch;
}

describe("selectRegion", () => {
  beforeEach(() => {
    stubRegionsFetch();
    process.stdin.isTTY = true;
  });

  afterEach(() => {
    process.stdin.isTTY = false;
  });

  test("US region is shown first", async () => {
    const ctx = makeContext(null);

    const promise = selectRegion(ctx, 123, "dev");

    await screen.next();
    const rendered = screen.getScreen();
    // US East should appear before EU West in the rendered output
    const usIndex = rendered.indexOf("US East");
    const euIndex = rendered.indexOf("EU West");
    expect(usIndex).toBeGreaterThanOrEqual(0);
    expect(euIndex).toBeGreaterThanOrEqual(0);
    expect(usIndex).toBeLessThan(euIndex);

    // Select the first option (US East) and resolve the promise
    screen.keypress("enter");
    const result = await promise;
    expect(result).toBe("aws-us-east-1");
  });

  test("unavailable regions are not displayed", async () => {
    const ctx = makeContext(null);

    const promise = selectRegion(ctx, 123, "dev");

    await screen.next();
    const rendered = screen.getScreen();
    expect(rendered).not.toContain("Asia Pacific (Singapore)");
    expect(rendered).toContain("US East (Virginia)");
    expect(rendered).toContain("EU West (Ireland)");

    screen.keypress("enter");
    await promise;
  });

  const availableRegions = testRegions.filter((r) => r.available);

  test.each(availableRegions.map((r, i) => ({ ...r, downPresses: i })))(
    "selecting option at position $downPresses returns the correct region",
    async ({ downPresses }) => {
      const ctx = makeContext(null);

      const promise = selectRegion(ctx, 123, "dev");

      await screen.next();
      for (let i = 0; i < downPresses; i++) {
        screen.keypress("down");
      }

      // Check which option is currently highlighted on screen,
      // then find the matching region to know what value to expect.
      const rendered = screen.getScreen();
      const selectedRegion = testRegions.find(
        (r) => r.available && rendered.includes(`❯ ${r.displayName}`),
      );
      expect(selectedRegion).toBeDefined();

      screen.keypress("enter");
      const result = await promise;
      expect(result).toBe(selectedRegion!.name);
    },
  );
});
