import { describe, expect, test } from "vitest";
import { Value } from "../../values/index.js";
import { BaseConvexClientInterface, Transition } from "./client.js";
import { PaginatedQueryClient } from "./paginated_query_client.js";
import { QueryToken } from "./udf_path_utils.js";
import { Long } from "../../vendor/long.js";

class TestBaseClient {
  private transitionHandler: ((transition: Transition) => void) | undefined;
  private nextToken = 0;
  readonly subscriptions: Array<{
    queryToken: QueryToken;
    args: Record<string, Value>;
  }> = [];
  readonly results = new Map<QueryToken, Value>();

  addOnTransitionHandler(handler: (transition: Transition) => void) {
    this.transitionHandler = handler;
    return () => {
      this.transitionHandler = undefined;
    };
  }

  subscribe(_name: string, args: Record<string, Value> = {}) {
    const queryToken = `query-${this.nextToken++}` as QueryToken;
    this.subscriptions.push({ queryToken, args });
    return { queryToken, unsubscribe: () => {} };
  }

  localQueryResultByToken(queryToken: QueryToken) {
    return this.results.get(queryToken);
  }

  emitTransition(queryToken: QueryToken) {
    this.transitionHandler?.({
      queries: [
        {
          token: queryToken,
          modification: { kind: "Updated", result: undefined },
        },
      ],
      reflectedMutations: [],
      timestamp: Long.fromNumber(0),
    });
  }
}

describe("PaginatedQueryClient", () => {
  test("splitting a later page preserves its start cursor", () => {
    const baseClient = new TestBaseClient();
    const paginatedClient = new PaginatedQueryClient(
      baseClient as unknown as BaseConvexClientInterface,
      () => {},
    );
    const { paginatedQueryToken } = paginatedClient.subscribe(
      "messages:list",
      {},
      { initialNumItems: 2, id: 1 },
    );

    const firstPage = baseClient.subscriptions[0];
    baseClient.results.set(firstPage.queryToken, {
      page: [1, 2],
      isDone: false,
      continueCursor: "page-1-end",
    });
    baseClient.emitTransition(firstPage.queryToken);

    const result = paginatedClient.localQueryResultByToken(paginatedQueryToken);
    expect(result?.status).toBe("CanLoadMore");
    expect(result?.loadMore(2)).toBe(true);

    const secondPage = baseClient.subscriptions[1];
    expect(secondPage.args.paginationOpts).toMatchObject({
      cursor: "page-1-end",
    });
    baseClient.results.set(secondPage.queryToken, {
      page: [3, 4],
      isDone: false,
      continueCursor: "page-2-end",
      splitCursor: "page-2-middle",
      pageStatus: "SplitRequired",
    });
    baseClient.emitTransition(secondPage.queryToken);

    expect(baseClient.subscriptions[2].args.paginationOpts).toMatchObject({
      cursor: "page-1-end",
      endCursor: "page-2-middle",
    });
    expect(baseClient.subscriptions[3].args.paginationOpts).toMatchObject({
      cursor: "page-2-middle",
      endCursor: "page-2-end",
    });
  });
});
