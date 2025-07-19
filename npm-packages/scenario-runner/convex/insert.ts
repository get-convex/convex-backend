import randomWords from "random-words";
import { DatabaseWriter, mutation } from "./_generated/server";
import { DataModel } from "./_generated/dataModel";
import { TableNamesInDataModel } from "convex/server";
import { v } from "convex/values";
import { rand } from "./common";

async function insertMessageHelper(
  db: DatabaseWriter,
  channel: string,
  timestamp: number,
  rand: number,
  ballastCount: number,
  count: number,
  table: TableNamesInDataModel<DataModel>,
): Promise<void> {
  for (let i = 0; i < count; i++) {
    const body = randomWords({ exactly: 1 }).join(" ");
    const ballastArray = [];
    for (let j = 0; j < ballastCount; j++) {
      ballastArray.push(j);
    }

    await db.insert(table, {
      channel,
      timestamp,
      rand,
      ballastArray,
      body,
    });
  }
}
export const insertMessage = mutation({
  handler: async ({ db }): Promise<void> => {
    await insertMessageHelper(
      db,
      "global",
      Date.now(),
      rand(),
      0,
      1,
      "messages",
    );
  },
});

export const insertMessageWithSearch = mutation({
  handler: async ({ db }): Promise<void> => {
    await insertMessageHelper(
      db,
      "global",
      Date.now(),
      rand(),
      0,
      1,
      "messages_with_search",
    );
  },
});

export const insertLargeMessage = mutation({
  args: {},
  handler: async ({ db }) => {
    await insertMessageHelper(
      db,
      "large",
      Date.now(),
      rand(),
      4000,
      100,
      "messages_with_search",
    );
  },
});

export const insertMessageWithArgs = mutation({
  args: {
    channel: v.string(),
    timestamp: v.number(),
    rand: v.number(),
    ballastCount: v.number(),
    count: v.number(),
    table: v.string(),
  },
  handler: async (
    ctx,
    { channel, timestamp, rand, ballastCount, count, table },
  ) => {
    await insertMessageHelper(
      ctx.db,
      channel,
      timestamp,
      rand,
      ballastCount,
      count,
      table as TableNamesInDataModel<DataModel>,
    );
  },
});

export const insertMessagesWithArgs = mutation({
  args: {
    channel: v.string(),
    timestamp: v.number(),
    rand: v.number(),
    ballastCount: v.number(),
    count: v.number(),
    table: v.string(),
    n: v.number(),
  },
  handler: async (
    ctx,
    { channel, timestamp, rand, ballastCount, count, table, n },
  ) => {
    for (let i = 0; i < n; i++) {
      await insertMessageHelper(
        ctx.db,
        channel,
        timestamp,
        rand,
        ballastCount,
        count,
        table as TableNamesInDataModel<DataModel>,
      );
    }
  },
});
