/**
 * @vitest-environment custom-vitest-enviroment.ts
 */
/* eslint-disable @typescript-eslint/ban-types */
import { expect, vi, test, describe, beforeEach } from "vitest";
import { act, renderHook } from "@testing-library/react";
import React from "react";

import {
  anyApi,
  FunctionReference,
  makeFunctionReference,
  PaginationOptions,
  PaginationResult,
} from "../server/index.js";
import { assert, Equals } from "../test/type_testing.js";
import { Value } from "../values/index.js";
import { ConvexProvider, ConvexReactClient } from "./client.js";
import {
  PaginatedQueryArgs,
  resetPaginationId,
  usePaginatedQuery,
} from "./use_paginated_query.js";
import { PaginatedQueryItem } from "./use_paginated_query.js";

const address = "https://127.0.0.1:3001";

type Props = { onError: (e: Error) => void; children: any };
class ErrorBoundary extends React.Component<Props> {
  state: { error: Error | undefined } = { error: undefined };
  onError: (e: Error) => void;

  constructor(props: Props) {
    super(props);
    this.onError = props.onError;
  }

  componentDidCatch(error: Error) {
    this.onError(error);
    return { error };
  }

  render() {
    if (this.state.error) {
      return this.state.error.toString();
    }

    return this.props.children;
  }
}

test.each([
  {
    options: undefined,
    expectedError:
      "Error: `options.initialNumItems` must be a positive number. Received `undefined`.",
  },
  {
    options: {},
    expectedError:
      "Error: `options.initialNumItems` must be a positive number. Received `undefined`.",
  },
  {
    options: { initialNumItems: -1 },
    expectedError:
      "Error: `options.initialNumItems` must be a positive number. Received `-1`.",
  },
  {
    options: { initialNumItems: "wrongType" },
    expectedError:
      "Error: `options.initialNumItems` must be a positive number. Received `wrongType`.",
  },
])("Throws an error when options is $options", ({ options, expectedError }) => {
  const convexClient = new ConvexReactClient(address);
  let lastError: Error | undefined = undefined;
  function updateError(e: Error) {
    lastError = e;
  }

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <ErrorBoundary onError={updateError}>
      <ConvexProvider client={convexClient}>{children}</ConvexProvider>
    </ErrorBoundary>
  );

  renderHook(
    () =>
      // @ts-expect-error We're testing user programming errors
      usePaginatedQuery(makeFunctionReference<"query">("myQuery"), {}, options),
    {
      wrapper,
    },
  );
  expect(lastError).not.toBeUndefined();
  expect(lastError!.toString()).toEqual(expectedError);
});

test.skip("Returns nothing when args are 'skip'", () => {
  const convexClient = new ConvexReactClient(address);
  const watchQuerySpy = vi.spyOn(convexClient, "watchQuery");
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <ConvexProvider client={convexClient}>{children}</ConvexProvider>
  );

  const { result } = renderHook(
    () =>
      usePaginatedQuery(makeFunctionReference<"query">("myQuery"), "skip", {
        initialNumItems: 10,
      }),
    { wrapper },
  );

  expect(watchQuerySpy.mock.calls).toEqual([]);
  expect(result.current).toMatchObject({
    isLoading: true,
    results: [],
    status: "LoadingFirstPage",
  });
});

test.skip("Initially returns LoadingFirstPage", () => {
  const convexClient = new ConvexReactClient(address);
  const watchQuerySpy = vi.spyOn(convexClient, "watchQuery");
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <ConvexProvider client={convexClient}>{children}</ConvexProvider>
  );

  const { result } = renderHook(
    () =>
      usePaginatedQuery(
        makeFunctionReference<"query">("myQuery"),
        {},
        { initialNumItems: 10 },
      ),
    { wrapper },
  );

  expect(watchQuerySpy.mock.calls[1]).toEqual([
    makeFunctionReference("myQuery"),
    {
      paginationOpts: {
        cursor: null,
        id: expect.anything(),
        numItems: 10,
      },
    },
    { journal: undefined },
  ]);
  expect(result.current).toMatchObject({
    isLoading: true,
    results: [],
    status: "LoadingFirstPage",
  });
});

test.skip("Updates to a new query if query name or args change", () => {
  const convexClient = new ConvexReactClient(address);
  const watchQuerySpy = vi.spyOn(convexClient, "watchQuery");

  let args: [
    query: FunctionReference<"query">,
    args: Record<string, Value>,
    options: { initialNumItems: number },
  ] = [makeFunctionReference("myQuery"), {}, { initialNumItems: 10 }];
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <ConvexProvider client={convexClient}>{children}</ConvexProvider>
  );

  const { rerender } = renderHook(() => usePaginatedQuery(...args), {
    wrapper,
  });

  // Starts with just the initial query.
  expect(watchQuerySpy.mock.calls.length).toBe(3);
  expect(watchQuerySpy.mock.calls[1]).toEqual([
    makeFunctionReference("myQuery"),
    {
      paginationOpts: {
        cursor: null,
        id: expect.anything(),
        numItems: 10,
      },
    },
    { journal: undefined },
  ]);

  // If we change the query name, we get a new call.
  args = [
    makeFunctionReference<"query">("myQuery2"),
    {},
    { initialNumItems: 10 },
  ];
  rerender();
  expect(watchQuerySpy.mock.calls.length).toBe(6);
  expect(watchQuerySpy.mock.calls[4]).toEqual([
    makeFunctionReference("myQuery2"),
    {
      paginationOpts: {
        cursor: null,
        id: expect.anything(),
        numItems: 10,
      },
    },
    { journal: undefined },
  ]);

  // If we add an arg, it also updates.
  args = [
    makeFunctionReference("myQuery2"),
    { someArg: 123 },
    { initialNumItems: 10 },
  ];
  rerender();
  expect(watchQuerySpy.mock.calls.length).toBe(9);
  expect(watchQuerySpy.mock.calls[7]).toEqual([
    makeFunctionReference("myQuery2"),
    {
      paginationOpts: { cursor: null, id: expect.anything(), numItems: 10 },
      someArg: 123,
    },
    { journal: undefined },
  ]);

  // Updating to a new arg object that serializes the same thing doesn't increase
  // the all count.
  args = [
    makeFunctionReference("myQuery2"),
    { someArg: 123 },
    { initialNumItems: 10 },
  ];
  rerender();
  expect(watchQuerySpy.mock.calls.length).toBe(9);
});

describe.skip("usePaginatedQuery pages", () => {
  let client: ConvexReactClient;
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <ConvexProvider client={client}>{children}</ConvexProvider>
  );
  const query: FunctionReference<"query"> = makeFunctionReference("myQuery");
  const mockPage = (
    opts: PaginationOptions,
    retval: PaginationResult<unknown>,
  ) => {
    act(() => {
      // Set a query result with an optimistic update.
      // The mutation doesn't go through because the client's websocket isn't
      // connected, so the optimistic update persists.
      void client.mutation(
        anyApi.myMutation.default,
        {},
        {
          optimisticUpdate: (localStore) => {
            localStore.setQuery(
              anyApi.myQuery.default,
              {
                paginationOpts: { ...opts, id: 1 },
              },
              retval,
            );
          },
        },
      );
    });
  };

  beforeEach(() => {
    client = new ConvexReactClient(address);
    resetPaginationId();
  });

  test("loadMore", () => {
    const { result } = renderHook(
      () => usePaginatedQuery(query, {}, { initialNumItems: 1 }),
      { wrapper },
    );
    mockPage(
      {
        numItems: 1,
        cursor: null,
      },
      {
        page: ["item1"],
        continueCursor: "abc",
        isDone: false,
      },
    );
    mockPage(
      {
        numItems: 2,
        cursor: "abc",
      },
      {
        page: ["item2"],
        continueCursor: "def",
        isDone: true,
      },
    );
    expect(result.current.status).toStrictEqual("CanLoadMore");
    expect(result.current.results).toStrictEqual(["item1"]);
    act(() => {
      result.current.loadMore(2);
    });
    expect(result.current.status).toStrictEqual("Exhausted");
    expect(result.current.results).toStrictEqual(["item1", "item2"]);
  });

  test("single page updating", () => {
    const { result } = renderHook(
      () => usePaginatedQuery(query, {}, { initialNumItems: 1 }),
      { wrapper },
    );
    mockPage(
      {
        numItems: 1,
        cursor: null,
      },
      {
        page: ["item1"],
        continueCursor: "abc",
        isDone: true,
      },
    );
    expect(result.current.status).toStrictEqual("Exhausted");
    expect(result.current.results).toStrictEqual(["item1"]);
    mockPage(
      {
        numItems: 1,
        cursor: null,
      },
      {
        page: ["item2", "item3"],
        continueCursor: "def",
        isDone: true,
      },
    );
    expect(result.current.status).toStrictEqual("Exhausted");
    expect(result.current.results).toStrictEqual(["item2", "item3"]);
  });

  test("page split", () => {
    const { result } = renderHook(
      () => usePaginatedQuery(query, {}, { initialNumItems: 1 }),
      { wrapper },
    );
    mockPage(
      {
        numItems: 1,
        cursor: null,
      },
      {
        page: ["item1", "item2", "item3", "item4"],
        continueCursor: "abc",
        splitCursor: "mid",
        isDone: true,
      },
    );
    expect(result.current.status).toStrictEqual("Exhausted");
    expect(result.current.results).toStrictEqual([
      "item1",
      "item2",
      "item3",
      "item4",
    ]);
    mockPage(
      {
        numItems: 1,
        cursor: null,
        endCursor: "mid",
      },
      {
        page: ["item1S", "item2S"],
        continueCursor: "mid",
        isDone: false,
      },
    );
    mockPage(
      {
        numItems: 1,
        cursor: "mid",
        endCursor: "abc",
      },
      {
        page: ["item3S", "item4S"],
        continueCursor: "abc",
        isDone: true,
      },
    );
    expect(result.current.status).toStrictEqual("Exhausted");
    expect(result.current.results).toStrictEqual([
      "item1S",
      "item2S",
      "item3S",
      "item4S",
    ]);
  });
});

describe("PaginatedQueryArgs", () => {
  test("basic", () => {
    type MyQueryFunction = FunctionReference<
      "query",
      "public",
      { paginationOpts: PaginationOptions; property: string },
      PaginationResult<string>
    >;
    type Args = PaginatedQueryArgs<MyQueryFunction>;
    type ExpectedArgs = { property: string };
    assert<Equals<Args, ExpectedArgs>>();
  });
});

describe("PaginatedQueryItem", () => {
  test("interface return type", () => {
    interface ReturnType {
      property: string;
    }
    type MyQueryFunction = FunctionReference<
      "query",
      "public",
      { paginationOpts: PaginationOptions; property: string },
      PaginationResult<ReturnType>
    >;
    type ActualReturnType = PaginatedQueryItem<MyQueryFunction>;
    assert<Equals<ActualReturnType, ReturnType>>();
  });
});
