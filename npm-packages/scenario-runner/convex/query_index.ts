import { DatabaseReader, query } from "./_generated/server";
import { Id } from "./_generated/dataModel";
import { CACHE_BREAKER_ARGS, MessagesTable } from "./common";

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
  handler: async (
    { db },
    {
      channel,
      rand,
      limit,
      table,
    }: {
      channel: string;
      rand: number;
      limit: number;
      table: MessagesTable;
    },
  ) => {
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
  handler: async ({ db }, { id }: { id: Id<MessagesTable> }) => {
    return await db.get(id);
  },
});
