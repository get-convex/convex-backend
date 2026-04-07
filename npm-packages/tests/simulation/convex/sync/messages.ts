import { getCurrentUserIdOrThrow } from "../users";
import { s, streamQuery } from "./schema";

const t = s.table("messages", async (ctx, _id) => {
  const userId = await getCurrentUserIdOrThrow(ctx);
  const messageId = ctx.db.normalizeId("messages", _id);
  if (messageId === null) {
    console.log("message not found", _id);
    return null;
  }
  const message = await ctx.db.get(messageId);
  if (message === null) {
    return null;
  }
  if (message.author === userId) {
    return message;
  }
  const conversationMembers = await ctx.db
    .query("conversationMembers")
    .withIndex("by_conversation", (q) =>
      q.eq("conversationId", message.conversationId),
    )
    .collect();
  if (conversationMembers.find((m) => m.userId === userId) === undefined) {
    return null;
  }
  console.log("returning message", message);
  return message;
});

export const get = t.get;

export const by_conversation = t.index(
  "by_conversation",
  async function* (ctx, { key, inclusive, direction }) {
    const userId = await getCurrentUserIdOrThrow(ctx);
    const conversationStream = streamQuery(ctx, {
      table: "conversationMembers",
      index: "by_user_conversation",
      startIndexKey: [userId, ...key.slice(0, 1)],
      startInclusive: inclusive || key.length > 1,
      endIndexKey: [userId],
      endInclusive: true,
      order: direction,
    });
    for await (const [conversationMember, _indexKey] of conversationStream) {
      const conversationId = conversationMember.conversationId;
      const startKey = conversationId === key[0] ? key : [conversationId];
      console.log("startKey", key, startKey);
      const messageStream = streamQuery(ctx, {
        table: "messages",
        index: "by_conversation",
        startIndexKey: startKey as any[],
        startInclusive: inclusive,
        endIndexKey: [conversationId],
        endInclusive: true,
        order: direction,
      });
      for await (const [message, _indexKey] of messageStream) {
        console.log("message", message, message._id);
        yield message._id;
      }
    }
  },
);

export const by_creation_time = t.index(
  "by_creation_time",
  async function* (ctx, { key, inclusive, direction }) {
    const stream = streamQuery(ctx, {
      table: "messages",
      index: "by_creation_time",
      startIndexKey: key as any[],
      startInclusive: inclusive,
      order: direction,
    });
    for await (const [message, _indexKey] of stream) {
      yield message._id;
    }
  },
);
