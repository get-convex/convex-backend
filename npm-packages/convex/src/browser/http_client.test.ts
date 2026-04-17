import { afterEach, describe, expect, test, vi } from "vitest";

import { makeFunctionReference } from "../server/api.js";
import { ConvexHttpClient } from "./http_client.js";

function successResponse(value: unknown = null) {
  return {
    ok: true,
    status: 200,
    json: async () => ({
      status: "success",
      value,
      logLines: [],
    }),
    text: async () => "",
  } as Response;
}

describe("ConvexHttpClient invocation metadata", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  test("includes metadata in query requests", async () => {
    const fetchMock = vi.fn().mockResolvedValue(successResponse());
    const client = new ConvexHttpClient(
      "https://happy-animal-123.convex.cloud",
      {
        fetch: fetchMock as typeof fetch,
        logger: false,
      },
    );

    await client.query(
      makeFunctionReference<"query">("tasks:list"),
      {},
      {
        metadata: {
          correlationId: "corr_123",
          origin: "nuxt",
        },
      },
    );

    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock.mock.calls[0]?.[0]).toBe(
      "https://happy-animal-123.convex.cloud/api/query",
    );

    const request = fetchMock.mock.calls[0]?.[1] as RequestInit;
    expect(JSON.parse(request.body as string)).toMatchObject({
      path: "tasks:list",
      metadata: {
        correlationId: "corr_123",
        origin: "nuxt",
      },
    });
  });

  test("preserves metadata through the queued mutation path", async () => {
    const fetchMock = vi.fn().mockResolvedValue(successResponse());
    const client = new ConvexHttpClient(
      "https://happy-animal-123.convex.cloud",
      {
        fetch: fetchMock as typeof fetch,
        logger: false,
      },
    );

    await client.mutation(
      makeFunctionReference<"mutation">("tasks:create"),
      { title: "Hello" },
      {
        skipQueue: false,
        metadata: {
          correlationId: "corr_123",
          phase: "draft",
        },
      },
    );

    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock.mock.calls[0]?.[0]).toBe(
      "https://happy-animal-123.convex.cloud/api/mutation",
    );

    const request = fetchMock.mock.calls[0]?.[1] as RequestInit;
    expect(JSON.parse(request.body as string)).toMatchObject({
      path: "tasks:create",
      metadata: {
        correlationId: "corr_123",
        phase: "draft",
      },
    });
  });

  test("includes metadata in action requests", async () => {
    const fetchMock = vi.fn().mockResolvedValue(successResponse());
    const client = new ConvexHttpClient(
      "https://happy-animal-123.convex.cloud",
      {
        fetch: fetchMock as typeof fetch,
        logger: false,
      },
    );

    await client.action(
      makeFunctionReference<"action">("tasks:notify"),
      {},
      {
        metadata: {
          correlationId: "corr_123",
          phase: "notify",
        },
      },
    );

    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock.mock.calls[0]?.[0]).toBe(
      "https://happy-animal-123.convex.cloud/api/action",
    );

    const request = fetchMock.mock.calls[0]?.[1] as RequestInit;
    expect(JSON.parse(request.body as string)).toMatchObject({
      path: "tasks:notify",
      metadata: {
        correlationId: "corr_123",
        phase: "notify",
      },
    });
  });
});
