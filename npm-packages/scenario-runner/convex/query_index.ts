import { DatabaseReader, query } from "./_generated/server";
import { Id } from "./_generated/dataModel";
import { CACHE_BREAKER_ARGS, MessagesTable } from "./common";
import { v } from "convex/values";

export const queryMessages = query({
  args: CACHE_BREAKER_ARGS,
  handler: async ({ db }, { cacheBreaker }) => {
    // Query at a random offset in order to uniformly distributed queries that
    // don't hit the cache.
    return await queryMessagesHelper(
      db,
      "global",
      cacheBreaker,
      10,
      "messages",
    );
  },
});

export const queryMessagesWithSearch = query({
  args: CACHE_BREAKER_ARGS,
  handler: async ({ db }, { cacheBreaker }) => {
    // Query at a random offset in order to uniformly distributed queries that
    // don't hit the cache.
    return await queryMessagesHelper(
      db,
      "global",
      cacheBreaker,
      10,
      "messages_with_search",
    );
  },
});

export const queryMessagesWithArgs = query({
  args: {
    channel: v.string(),
    rand: v.number(),
    limit: v.number(),
    table: v.union(v.literal("messages"), v.literal("messages_with_search")),
  },
  handler: async ({ db }, { channel, rand, limit, table }) => {
    return await queryMessagesHelper(db, channel, rand, limit, table);
  },
});

function queryMessagesHelper(
  db: DatabaseReader,
  channel: string,
  rand: number,
  limit: number,
  table: MessagesTable,
) {
  return db
    .query(table)
    .withIndex("by_channel_rand", (q) =>
      q.eq("channel", channel).gte("rand", rand),
    )
    .take(limit);
}

export const queryMessagesById = query({
  args: {
    id: v.union(v.id("messages"), v.id("messages_with_search")),
  },
  handler: async ({ db }, { id }) => {
    // TODO(nicolas)
    // eslint-disable-next-line @convex-dev/explicit-table-ids
    return await db.get(id satisfies Id<MessagesTable>);
  },
});
