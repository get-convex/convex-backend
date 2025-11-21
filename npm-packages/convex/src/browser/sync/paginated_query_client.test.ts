import { test, expect, describe } from "vitest";

import { BaseConvexClient } from "./client.js";
import { QuerySetModification } from "./protocol.js";
import {
  nodeWebSocket,
  UpdateQueue,
  withInMemoryWebSocket,
} from "./client_node_test_helpers.js";
import { PaginationOptions } from "../../server/pagination.js";
import { Long } from "../../vendor/long.js";
import { anyApi } from "../../server/index.js";
import { PaginatedQueryClient } from "./paginated_query_client.js";

describe("BaseConvexClient paginated queries with server mocked at ws level", () => {
  test("Subscribing and adding a page", async () => {
    await withInMemoryWebSocket(async ({ address, receive, send }) => {
      const q = new UpdateQueue();
      const client = new BaseConvexClient(
        address,
        () => {}, // use paginated query client for all transitions
        {
          webSocketConstructor: nodeWebSocket,
          unsavedChangesWarning: false,
        },
      );

      const paginatedClient: PaginatedQueryClient = new PaginatedQueryClient(
        client,
        ({ queries, paginatedQueries }) =>
          q.onTransition(
            client,
            paginatedClient,
          )([
            ...queries.map((t) => t.token),
            ...paginatedQueries.map((t) => t.token),
          ]),
      );

      expect((await receive()).type).toEqual("Connect");
      expect((await receive()).type).toEqual("ModifyQuerySet");

      const subscribeResult = paginatedClient.subscribe(
        "myQuery",
        { channel: "general" },
        { initialNumItems: 3, id: 1 },
      );

      expect(subscribeResult).toHaveProperty("paginatedQueryToken");

      // Query for the first page
      const queryMessage = (await receive()) as QuerySetModification;
      expect(queryMessage.type).toEqual("ModifyQuerySet");
      expect(queryMessage.modifications).toHaveLength(1);
      expect(queryMessage.modifications[0].type).toEqual("Add");
      if (queryMessage.modifications[0].type !== "Add") throw new Error();
      expect(queryMessage.modifications[0].udfPath).toEqual("myQuery:default");

      // Should include pagination options in args
      const args = queryMessage.modifications[0].args[0] as unknown as {
        channel: string;
        paginationOpts: PaginationOptions;
      };
      expect(args).toHaveProperty("channel", "general");
      expect(args).toHaveProperty("paginationOpts");
      expect(args.paginationOpts).toHaveProperty("cursor", null);

      const result1 = paginatedClient.localQueryResult(
        "myQuery",
        { channel: "general" },
        { initialNumItems: 3, id: 1 },
      );
      expect(result1?.results).toEqual([]);
      expect(result1?.status).toEqual("LoadingFirstPage");

      send({
        type: "Transition",
        startVersion: {
          querySet: 0,
          identity: 0,
          ts: Long.fromNumber(0),
        },
        endVersion: {
          querySet: 1,
          identity: 0,
          ts: Long.fromNumber(100),
        },
        modifications: [
          {
            type: "QueryUpdated",
            queryId: queryMessage.modifications[0].queryId,
            value: {
              page: ["a", "b", "c"],
              isDone: false,
              continueCursor: "start after c",
              splitCursor: null,
              pageStatus: null,
            },
            logLines: [],
            journal: null,
          },
        ],
      });

      // That send should be enough to kick off a transition if we just wait for it.
      let i = 0;
      await q.awaitPromiseAtIndexWithTimeout(i++);

      const result2 = paginatedClient.localQueryResult(
        "myQuery",
        { channel: "general" },
        { initialNumItems: 3, id: 1 },
      );
      expect(result2?.results).toEqual(["a", "b", "c"]);
      expect(result2?.status).toEqual("CanLoadMore");

      result2!.loadMore(5);

      // just calling loadMore causes an update: now we're in LoadingMore state.
      const update2 = await q.awaitPromiseAtIndexWithTimeout(i++);
      expect(Object.keys(update2)).toHaveLength(1);
      expect(Object.values(update2)[0].status === "LoadingMore");

      // Query for the second page
      const queryMessage2 = (await receive()) as QuerySetModification;
      expect(queryMessage2.type).toEqual("ModifyQuerySet");
      expect(queryMessage2.modifications).toHaveLength(1);
      expect(queryMessage2.modifications[0].type).toEqual("Add");
      if (queryMessage2.modifications[0].type !== "Add") throw new Error();
      expect(queryMessage2.modifications[0].udfPath).toEqual("myQuery:default");
      expect(queryMessage2.modifications[0].args[0] as any).toEqual({
        channel: "general",
        paginationOpts: {
          cursor: "start after c",
          numItems: 5,
          id: 1,
        },
      });

      const localQueryResult = paginatedClient.localQueryResult(
        "myQuery",
        { channel: "general" },
        { initialNumItems: 3, id: 1 },
      );
      expect(localQueryResult?.status).toEqual("LoadingMore");
      expect(localQueryResult?.results).toEqual(["a", "b", "c"]);

      send({
        type: "Transition",
        startVersion: {
          querySet: 1,
          identity: 0,
          ts: Long.fromNumber(100),
        },
        endVersion: {
          querySet: 2,
          identity: 0,
          ts: Long.fromNumber(200),
        },
        modifications: [
          {
            type: "QueryUpdated",
            queryId: queryMessage2.modifications[0].queryId,
            value: {
              page: ["d", "e", "f"],
              isDone: false,
              continueCursor: "start after f",
              splitCursor: null,
              pageStatus: null,
            },
            logLines: [],
            journal: null,
          },
        ],
      });

      const update3 = await q.awaitPromiseAtIndexWithTimeout(i++);
      // Both the page query and the paginated query are updated
      expect(Object.keys(update3)).toHaveLength(2);

      // Let's add some elements
      send({
        type: "Transition",
        startVersion: {
          querySet: 2,
          identity: 0,
          ts: Long.fromNumber(200),
        },
        endVersion: {
          querySet: 2,
          identity: 0,
          ts: Long.fromNumber(300),
        },
        modifications: [
          {
            type: "QueryUpdated",
            queryId: queryMessage.modifications[0].queryId,
            value: {
              page: ["a", "b", "ba", "bb", "c"],
              isDone: false,
              continueCursor: "start after c",
              splitCursor: "after ba",
              pageStatus: "SplitRecommended",
            },
            logLines: [],
            journal: null,
          },
        ],
      });
      const update4 = await q.awaitPromiseAtIndexWithTimeout(i++);
      // Both the page query and the paginated query are updated
      expect(Object.keys(update4)).toHaveLength(2);

      const actual = paginatedClient.localQueryResult(
        "myQuery",
        { channel: "general" },
        { initialNumItems: 3, id: 1 },
      );
      expect(actual?.results).toEqual([
        "a",
        "b",
        "ba",
        "bb",
        "c",
        "d",
        "e",
        "f",
      ]);
      expect(actual?.status).toEqual("CanLoadMore");

      await client.close();
    });
  });
});

describe("BaseConvexClient paginated queries without connecting", () => {
  test("Page splitting with optimistic updates", async () => {
    // Use a non-existent address so the client can't connect
    // This allows optimistic updates to persist
    const address = "https://127.0.0.1:3001";
    const q = new UpdateQueue();

    const client = new BaseConvexClient(
      address,
      () => {}, // use the paginated client for all transitions
      {
        webSocketConstructor: nodeWebSocket,
        unsavedChangesWarning: false,
      },
    );
    const paginatedClient: PaginatedQueryClient = new PaginatedQueryClient(
      client,
      ({ queries, paginatedQueries }) =>
        q.onTransition(
          client,
          paginatedClient,
        )([
          ...queries.map((t) => t.token),
          ...paginatedQueries.map((t) => t.token),
        ]),
    );

    const mockPage = (
      opts: PaginationOptions,
      retval: {
        page: any[];
        continueCursor: string | null;
        isDone: boolean;
        splitCursor?: string | null;
        pageStatus?: "SplitRecommended" | null;
      },
    ) => {
      // Use an optimistic mutation to set query results
      void client.mutation(
        "myMutation",
        {},
        {
          optimisticUpdate: (localStore) => {
            localStore.setQuery(
              anyApi.myQuery.default,
              {
                channel: "general",
                paginationOpts: { ...opts, id: 1 },
              },
              retval,
            );
          },
        },
      );
    };

    // Subscribe to a paginated query
    const subscribeResult = paginatedClient.subscribe(
      "myQuery",
      { channel: "general" },
      { initialNumItems: 3, id: 1 },
    );

    expect(subscribeResult).toHaveProperty("paginatedQueryToken");

    // Initially should be loading
    let result = paginatedClient.localQueryResult(
      "myQuery",
      { channel: "general" },
      { initialNumItems: 3, id: 1 },
    );
    expect(result?.status).toEqual("LoadingFirstPage");

    // Mock first page - this should trigger a page split due to splitCursor
    mockPage(
      {
        numItems: 3,
        cursor: null,
      },
      {
        page: ["item1", "item2", "item3", "item4", "item5"],
        continueCursor: "after5",
        isDone: false,
        splitCursor: "after3",
        pageStatus: "SplitRecommended",
      },
    );

    // Wait for the transition to process
    await q.awaitPromiseAtIndexWithTimeout(0);

    // The splitting logic should have been triggered
    result = paginatedClient.localQueryResult(
      "myQuery",
      { channel: "general" },
      { initialNumItems: 3, id: 1 },
    );
    expect(result?.results).toEqual([
      "item1",
      "item2",
      "item3",
      "item4",
      "item5",
    ]);
    expect(result?.status).toEqual("CanLoadMore");

    // Mock the split pages - first half
    mockPage(
      {
        numItems: 3,
        cursor: null,
        endCursor: "after3",
      },
      {
        page: ["item1S", "item2S", "item3S"],
        continueCursor: "after3",
        isDone: false,
      },
    );

    // Mock the split pages - second half
    mockPage(
      {
        numItems: 3,
        cursor: "after3",
        endCursor: "after5",
      },
      {
        page: ["item4S", "item5S"],
        continueCursor: "after5",
        isDone: false,
      },
    );

    await q.awaitPromiseAtIndexWithTimeout(1);
    await q.awaitPromiseAtIndexWithTimeout(2);

    result = paginatedClient.localQueryResult(
      "myQuery",
      { channel: "general" },
      { initialNumItems: 3, id: 1 },
    );
    expect(result?.results).toEqual([
      "item1S",
      "item2S",
      "item3S",
      "item4S",
      "item5S",
    ]);
    expect(result?.status).toEqual("CanLoadMore");

    await client.close();
  });
});
