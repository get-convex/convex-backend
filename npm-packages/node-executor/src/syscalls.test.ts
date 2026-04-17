import { afterEach, describe, expect, test, vi } from "vitest";

import { ExecutionContext } from "./executor";
import { SyscallsImpl } from "./syscalls";

function makeExecutionContext(): ExecutionContext {
  return {
    requestId: "req_123",
    executionId: "exec_123",
    isRoot: true,
    parentScheduledJob: null,
    parentScheduledJobComponentId: null,
    invocationMetadata: {
      correlationId: "corr_123",
      origin: "nuxt",
    },
  };
}

function successResponse(value: unknown = null) {
  return {
    ok: true,
    status: 200,
    json: async () => ({
      status: "success",
      value,
    }),
    text: async () => "",
  } as Response;
}

function makeSyscalls() {
  return new SyscallsImpl(
    {
      canonicalizedPath: "tasks.ts",
      function: "run",
    },
    "lambda_123",
    "https://example.com",
    "callback-token",
    null,
    null,
    makeExecutionContext(),
    null,
  );
}

describe("SyscallsImpl invocation metadata", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  test("encodes inherited invocation metadata in callback headers", () => {
    const headers = makeSyscalls().headers("1.0");
    const encoded = headers["Convex-Invocation-Metadata"];

    expect(encoded).toBeDefined();
    expect(
      JSON.parse(Buffer.from(encoded!, "base64url").toString("utf8")),
    ).toEqual({
      correlationId: "corr_123",
      origin: "nuxt",
    });
  });

  test("returns the current invocation context through the syscall", async () => {
    const result = JSON.parse(
      await makeSyscalls().asyncSyscall("1.0/getInvocationContext", "{}"),
    );

    expect(result).toEqual({
      requestId: "req_123",
      executionId: "exec_123",
      isRoot: true,
      parentScheduledJob: null,
      parentScheduledJobComponentId: null,
      metadata: {
        correlationId: "corr_123",
        origin: "nuxt",
      },
    });
  });

  test("sends override metadata in the callback body while preserving inherited headers", async () => {
    const fetchMock = vi.fn().mockResolvedValue(successResponse());
    vi.stubGlobal("fetch", fetchMock as typeof fetch);

    await makeSyscalls().syscallQuery(
      JSON.stringify({
        requestId: "lambda_123",
        version: "1.0",
        name: "tasks:list",
        args: {},
        metadata: {
          phase: "draft",
        },
      }),
    );

    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect((fetchMock.mock.calls[0]?.[0] as URL).href).toBe(
      "https://example.com/api/actions/query",
    );

    const request = fetchMock.mock.calls[0]?.[1] as RequestInit;
    const headers = request.headers as Record<string, string>;
    expect(
      JSON.parse(
        Buffer.from(
          headers["Convex-Invocation-Metadata"],
          "base64url",
        ).toString("utf8"),
      ),
    ).toEqual({
      correlationId: "corr_123",
      origin: "nuxt",
    });
    expect(JSON.parse(request.body as string)).toMatchObject({
      path: "tasks:list",
      metadata: {
        phase: "draft",
      },
    });
  });
});
