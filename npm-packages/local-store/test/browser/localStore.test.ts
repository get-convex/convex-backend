import { expect, test } from "vitest";
import { CopyOnWriteLocalStore } from "../../browser/core/localStore";
import { sync as syncSchema } from "../../../simulation/convex/sync/schema";
import { anyApi, defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";
import { Writes } from "../../browser/core/protocol";
import {
  LOG2_PAGE_SIZE,
  SingleIndexRangeExecutor,
  indexRangeUnbounded,
} from "../../browser/core/paginator";
import { createQueryToken } from "../../shared/queryTokens";
import { PageArguments, PageResult } from "../../shared/types";

export const sync = defineSchema({
  // specific to current user
  conversations: defineTable({
    _id: v.string(),
    latestMessageTime: v.number(),
    emoji: v.optional(v.string()),
    users: v.array(v.id("users")),
    hasUnreadMessages: v.boolean(),
  }).index("by_priority", ["hasUnreadMessages", "latestMessageTime"]),
});

test("applyWrites", async () => {
  const localStore = new CopyOnWriteLocalStore(syncSchema);
  const pageArguments: PageArguments = {
    syncTableName: "conversations",
    index: "by_priority",
    target: { kind: "successor", value: [] },
    log2PageSize: LOG2_PAGE_SIZE,
  };
  const pageResult: PageResult = {
    results: [],
    lowerBound: { kind: "predecessor", value: [] },
    upperBound: { kind: "successor", value: [] },
  };
  localStore.ingest([
    {
      tableName: "conversations",
      indexName: "by_priority",
      convexSubscriptionId: createQueryToken(
        anyApi.sync.conversations.by_priority,
        pageArguments,
      ),
      state: {
        kind: "loaded",
        value: pageResult,
      },
    },
  ]);
  const writesA = new Writes();
  writesA.set("conversations", "1" as any, {
    _id: "1",
    latestMessageTime: 1,
    hasUnreadMessages: true,
    emoji: "A",
    users: [],
  });
  localStore.applyWrites(writesA);
  const paginator = new SingleIndexRangeExecutor(
    {
      tableName: "conversations",
      indexName: "by_priority",
      indexRangeBounds: indexRangeUnbounded,
      count: 100,
      order: "desc",
    },
    syncSchema,
    localStore,
  );
  const result = paginator.tryFulfill();
  expect(result.state).toEqual("fulfilled");

  const results =
    result.state === "fulfilled" ? result.results.map((r) => r.emoji) : [];
  expect(results).toEqual(["A"]);

  const writesB = new Writes();
  writesB.set("conversations", "2" as any, {
    _id: "2",
    latestMessageTime: 2,
    hasUnreadMessages: true,
    emoji: "B",
    users: [],
  });
  localStore.applyWrites(writesB);
  const paginatorB = new SingleIndexRangeExecutor(
    {
      tableName: "conversations",
      indexName: "by_priority",
      indexRangeBounds: indexRangeUnbounded,
      count: 100,
      order: "desc",
    },
    syncSchema,
    localStore,
  );
  const resultB = paginatorB.tryFulfill();
  expect(resultB.state).toEqual("fulfilled");
  const resultsB =
    resultB.state === "fulfilled" ? resultB.results.map((r) => r.emoji) : [];
  expect(resultsB).toEqual(["B", "A"]);
});
