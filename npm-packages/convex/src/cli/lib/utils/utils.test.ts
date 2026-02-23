import { describe, test, expect, vi, afterEach } from "vitest";
import type { BigBrainAuth, Context } from "../../../bundler/context.js";
import {
  bigBrainFetch,
  bigBrainAPIMaybeThrows,
  BIG_BRAIN_URL,
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

    const fetch = await bigBrainFetch(ctx);
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

    const fetch = await bigBrainFetch(ctx);
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

    const fetch = await bigBrainFetch(ctx);
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

    const fetch = await bigBrainFetch(ctx);
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

    const fetch = await bigBrainFetch(ctx);
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

    const fetch = await bigBrainFetch(ctx);
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
