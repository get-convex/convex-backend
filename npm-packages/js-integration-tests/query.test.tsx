/**
 * @jest-environment jsdom
 */

import { ConvexHttpClient } from "convex/browser";
import {
  ConvexReactClient,
  usePaginatedQuery,
  ConvexProvider,
} from "convex/react";
import React from "react";
import { PaginationResult } from "convex/server";
import { api } from "./convex/_generated/api";
import { awaitQueryResult } from "./test_helpers";
import { act, renderHook } from "@testing-library/react";
import { Doc } from "./convex/_generated/dataModel";
import { deploymentUrl } from "./common";

// eslint-disable-next-line jest/no-disabled-tests
describe.skip("ConvexHttpClient", () => {
  let client: ConvexHttpClient;

  beforeEach(() => {
    client = new ConvexHttpClient(deploymentUrl);
  });

  afterEach(async () => {
    await client.mutation(api.cleanUp.default);
  });

  test("basic query", async () => {
    const doc1 = await client.mutation(api.messages.sendMessage, {
      channel: "channel",
      text: "hello!",
    });
    await client.mutation(api.messages.sendMessage, {
      channel: "otherChannel",
      text: "hi!",
    });
    const doc3 = await client.mutation(api.messages.sendMessage, {
      channel: "channel",
      text: "there!",
    });

    const results = await client.query(api.messages.listMessages, {
      channel: "channel",
    });

    expect(results).toMatchObject([doc1, doc3]);
  });

  test("paginated query", async () => {
    const expected = [];
    const msgs = [];

    const msgChannels = [
      //        idx   foo?     fixed page size     variable page size
      "bar", // 0              ┬                   ┬
      "foo", //       *        │ 1                 │1
      "foo", //       *        │ 2                 │2
      "bar", //                │                   ┴
      "foo", //       *        │ 3                 ┬1
      "bar", // 5              │                   │
      "bar", //                │                   │
      "bar", //                │                   ┴
      "bar", //                │                   ┬
      "bar", //                │                   │
      "bar", // 10             │                   │
      "bar", //                │                   ┴
      "bar", //                │                   ┬
      "foo", //       *        ┴ 4                 │1
      "foo", //       *        ┬ 1                 │2
      "bar", // 15             │                   ┴
      "foo", //       *        │ 2                 ┬1
      "foo", //       *        │ 3                 │2
      "bar", //                │                   │
      "foo", //       *        ┴ 4                 ┴3
      "foo", // 20    *        ─ 1                 ─1
    ];

    for (let i = 0; i < msgChannels.length; i++) {
      const channel = msgChannels[i];
      const doc = await client.mutation(api.messages.sendMessage, {
        channel,
        text: i.toString(),
      });
      msgs.push(doc);
      if (channel === "foo") {
        expected.push(doc);
      }
    }

    const readToEnd = async (opts: {
      numItems: number;
      maximumRowsRead?: number;
      maximumBytesRead?: number;
    }) => {
      const results = [];
      const pages = [];
      let cursor = null;

      // eslint-disable-next-line no-constant-condition
      while (true) {
        const {
          page,
          isDone,
          continueCursor,
        }: PaginationResult<Doc<"messages">> = await client.query(
          api.messages.paginatedListMessagesByChannel,
          {
            paginationOpts: { cursor, ...opts },
            channel: "foo",
          },
        );
        cursor = continueCursor;

        results.push(...page);
        pages.push(page);

        if (isDone) {
          break;
        }
      }

      return {
        results: results,
        pages: pages,
      };
    };

    // Read-to-end with an exact page size should always fill the page if it can.
    let result = await readToEnd({ numItems: 4 });
    expect(result.results).toMatchObject(expected);
    // See diagram above.
    expect(result.pages).toMatchObject([
      [msgs[1], msgs[2], msgs[4], msgs[13]],
      [msgs[14], msgs[16], msgs[17], msgs[19]],
      [msgs[20]],
    ]);

    // Read-to-end with variable pages should page over the index scan instead.
    result = await readToEnd({ maximumRowsRead: 4, numItems: 100 });
    expect(result.results).toMatchObject(expected);
    // See diagram above.
    expect(result.pages).toMatchObject([
      [msgs[1], msgs[2]],
      [msgs[4]],
      [],
      [msgs[13], msgs[14]],
      [msgs[16], msgs[17], msgs[19]],
      [msgs[20]],
    ]);

    // Read-to-end with pages that vary based on size.
    // Each document is ~100 bytes, so this should chunk them nicely.
    result = await readToEnd({ maximumBytesRead: 500, numItems: 100 });
    expect(result.results).toMatchObject(expected);
    // Don't assert precisely on the size, but make sure it's getting broken up
    // into more than 1 page and less than 1 result per page.
    expect(result.pages.length).toBeGreaterThan(1);
    expect(result.pages.length).toBeLessThan(expected.length);
  }, 20000);
});

// eslint-disable-next-line jest/no-disabled-tests
describe.skip("ConvexReactClient", () => {
  let client: ConvexReactClient;

  beforeEach(() => {
    client = new ConvexReactClient(deploymentUrl);
  });
  afterEach(async () => {
    await client.mutation(api.cleanUp.default);
    await client.close();
  });

  function messageNamesFromResult(
    paginationResult: PaginationResult<Doc<"messages">>,
  ) {
    return paginationResult.page.map((message) => message.text);
  }

  test("paginated query pages can shrink", async () => {
    await client.mutation(api.messages.sendMessage, {
      channel: "channelA",
      text: "message1",
    });
    const message2 = await client.mutation(api.messages.sendMessage, {
      channel: "channelB",
      text: "message2",
    });
    await client.mutation(api.messages.sendMessage, {
      channel: "channelC",
      text: "message3",
    });

    const watchMessagesByChannel = client.watchQuery(
      api.messages.paginatedListMessagesByCreationTime,
      { paginationOpts: { cursor: null, numItems: 2 } },
    );

    // Load our paginated query. We asked for 2 items so intially there should be 2.
    const result1 = awaitQueryResult(watchMessagesByChannel, () => true);
    expect(messageNamesFromResult(await result1)).toStrictEqual([
      "message1",
      "message2",
    ]);

    // When we delete item two, the query should retain it's end cursor
    // and only have a single item.
    const result2 = awaitQueryResult(
      watchMessagesByChannel,
      (result) => result.page.length === 1,
    );
    await client.mutation(api.removeObject.default, { id: message2!._id });
    expect(messageNamesFromResult(await result2)).toStrictEqual(["message1"]);
  });

  test("paginated query pages can grow", async () => {
    await client.mutation(api.messages.sendMessage, {
      channel: "channelA",
      text: "message1",
    });
    await client.mutation(api.messages.sendMessage, {
      channel: "channelC",
      text: "message2",
    });
    await client.mutation(api.messages.sendMessage, {
      channel: "channelD",
      text: "message3",
    });

    const watchMessagesByChannel = client.watchQuery(
      api.messages.paginatedListMessagesByCreationTime,
      { paginationOpts: { cursor: null, numItems: 2 } },
    );

    // Load our paginated query. We asked for 2 items so intially there should be 2.
    const result1 = awaitQueryResult(watchMessagesByChannel, () => true);
    expect(messageNamesFromResult(await result1)).toStrictEqual([
      "message1",
      "message2",
    ]);

    // When we add a new item that falls in our page, the query should grow to contain it.
    const result2 = awaitQueryResult(
      watchMessagesByChannel,
      (result) => result.page.length === 3,
    );
    await client.mutation(api.messages.sendMessage, {
      channel: "channelB",
      text: "message4",
    });
    expect(messageNamesFromResult(await result2)).toStrictEqual([
      "message1",
      "message4",
      "message2",
    ]);
  });

  test("usePaginatedQuery hook splits pages", async () => {
    const wrapper = ({ children }: any) => (
      <ConvexProvider client={client}>{children}</ConvexProvider>
    );
    const { result } = renderHook(
      () =>
        usePaginatedQuery(
          api.messages.paginatedListMessagesWithExplicitPages,
          {},
          { initialNumItems: 1 },
        ),
      {
        wrapper,
      },
    );
    expect(result.current.results).toStrictEqual([]);
    await act(async () => {
      await client.mutation(api.messages.sendMessage, {
        channel: "channelA",
        text: "message1",
      });
    });

    expect(result.current.results.map((m) => [m.text, m.i])).toStrictEqual([
      ["message1", 0],
    ]);

    await act(async () => {
      await client.mutation(api.messages.sendMessage, {
        channel: "channelC",
        text: "message2",
      });
      await client.mutation(api.messages.sendMessage, {
        channel: "channelD",
        text: "message3",
      });
    });

    expect(result.current.results.map((m) => [m.text, m.i])).toStrictEqual([
      ["message1", 0],
      ["message2", 1],
      ["message3", 2],
    ]);

    await act(async () => {
      await client.mutation(api.messages.sendMessage, {
        channel: "channelE",
        text: "message4",
      });
    });

    expect(result.current.results.map((m) => [m.text, m.i])).toStrictEqual([
      ["message1", 0],
      ["message2", 1],
      ["message3", 0],
      ["message4", 1],
    ]);
  });

  test("usePaginatedQuery hook SplitRecommended", async () => {
    const wrapper = ({ children }: any) => (
      <ConvexProvider client={client}>{children}</ConvexProvider>
    );
    // Split is recommended because maximumRowsRead is 3.
    const { result } = renderHook(
      () =>
        usePaginatedQuery(
          api.messages.paginatedListMessagesMaxRows,
          {},
          { initialNumItems: 5 },
        ),
      {
        wrapper,
      },
    );
    expect(result.current.results).toStrictEqual([]);
    await act(async () => {
      await client.mutation(api.messages.sendMessage, {
        channel: "channelA",
        text: "message1",
      });
    });

    expect(result.current.results.map((m) => [m.text, m.i])).toStrictEqual([
      ["message1", 0],
    ]);

    await act(async () => {
      await client.mutation(api.messages.sendMessage, {
        channel: "channelC",
        text: "message2",
      });
      await client.mutation(api.messages.sendMessage, {
        channel: "channelD",
        text: "message3",
      });
      await client.mutation(api.messages.sendMessage, {
        channel: "channelE",
        text: "message4",
      });
    });
    expect(result.current.results.map((m) => [m.text, m.i])).toStrictEqual([
      ["message1", 0],
      ["message2", 1],
      ["message3", 2],
      ["message4", 3],
    ]);
    await act(async () => {
      // At this point the split is executing and we just need to wait for it.
      await client.mutation(api.messages.sendMessage, {
        channel: "channelF",
        text: "message5",
      });
    });

    expect(result.current.results.map((m) => [m.text, m.i])).toStrictEqual([
      ["message1", 0],
      ["message2", 1],
      ["message3", 2],
      ["message4", 0],
      ["message5", 1],
    ]);
  });

  test("if the result is empty, paginated queries subscribe to everything", async () => {
    const watchMessagesByChannel = client.watchQuery(
      api.messages.paginatedListMessagesByCreationTime,
      { paginationOpts: { cursor: null, numItems: 1 } },
    );
    // Load our paginated query. We didn't set up any data so there should be
    // nothing.
    const result1 = awaitQueryResult(watchMessagesByChannel, () => true);
    expect(messageNamesFromResult(await result1)).toStrictEqual([]);

    // If we add 2 more items, they should both appear in the result even though
    // we only asked for a numItems of 1 because we're subscribed to everything.
    const result2 = awaitQueryResult(
      watchMessagesByChannel,
      (result) => result.page.length === 2,
    );

    await client.mutation(api.messages.sendMessage, {
      channel: "channelA",
      text: "message1",
    });
    await client.mutation(api.messages.sendMessage, {
      channel: "channelB",
      text: "message2",
    });
    expect(messageNamesFromResult(await result2)).toStrictEqual([
      "message1",
      "message2",
    ]);
  });

  test("paginated query pageStatus", async () => {
    // Each document is about 1KB
    const text = "abcdefghij".repeat(100);
    await client.mutation(api.messages.sendMessage, {
      channel: "channelA",
      text,
    });
    await client.mutation(api.messages.sendMessage, {
      channel: "channelC",
      text,
    });
    await client.mutation(api.messages.sendMessage, {
      channel: "channelD",
      text,
    });

    const splitNotRecommended = client.watchQuery(
      api.messages.paginatedListMessagesByCreationTime,
      { paginationOpts: { cursor: null, numItems: 1, maximumRowsRead: 3 } },
    );
    const result1 = await awaitQueryResult(splitNotRecommended, () => true);
    expect(result1.page.length).toStrictEqual(1);
    expect(result1.pageStatus).toStrictEqual(null);

    const almostTooManyRows = client.watchQuery(
      api.messages.paginatedListMessagesByCreationTime,
      { paginationOpts: { cursor: null, numItems: 3, maximumRowsRead: 3 } },
    );
    const result2 = await awaitQueryResult(almostTooManyRows, () => true);
    expect(result2.page.length).toStrictEqual(3);
    expect(result2.pageStatus).toStrictEqual("SplitRecommended");

    const tooManyRows = client.watchQuery(
      api.messages.paginatedListMessagesByCreationTime,
      { paginationOpts: { cursor: null, numItems: 3, maximumRowsRead: 2 } },
    );
    const result3 = await awaitQueryResult(tooManyRows, () => true);
    expect(result3.page.length).toStrictEqual(2);
    expect(result3.pageStatus).toStrictEqual("SplitRequired");

    const almostTooManyBytes = client.watchQuery(
      api.messages.paginatedListMessagesByCreationTime,
      { paginationOpts: { cursor: null, numItems: 3, maximumBytesRead: 3000 } },
    );
    const result4 = await awaitQueryResult(almostTooManyBytes, () => true);
    expect(result4.page.length).toStrictEqual(3);
    expect(result4.pageStatus).toStrictEqual("SplitRecommended");

    const tooManyBytes = client.watchQuery(
      api.messages.paginatedListMessagesByCreationTime,
      { paginationOpts: { cursor: null, numItems: 3, maximumBytesRead: 2000 } },
    );
    const result5 = await awaitQueryResult(tooManyBytes, () => true);
    expect(result5.page.length).toStrictEqual(2);
    expect(result5.pageStatus).toStrictEqual("SplitRequired");
  });
});
