import { Doc } from "../_generated/dataModel";
import { QueryCtx } from "../_generated/server";
import { getCurrentUserIdOrThrow } from "../users";
import { s, streamQuery } from "./schema";

const convertConversation = async (
  ctx: QueryCtx,
  c: Doc<"conversationMembers">,
) => {
  const conversation = await ctx.db.get(c.conversationId);
  const members = await ctx.db
    .query("conversationMembers")
    .withIndex("by_conversation", (q) =>
      q.eq("conversationId", c.conversationId),
    )
    .collect();
  return {
    _id: c.conversationId,
    _creationTime: c._creationTime,
    latestMessageTime: c.latestMessageTime,
    emoji: conversation?.emoji,
    users: members.map((m) => m.userId),
    hasUnreadMessages: c.hasUnreadMessages,
  };
};

const table = s.table("conversations", async (ctx, _id) => {
  const userId = await getCurrentUserIdOrThrow(ctx);
  const conversationId = ctx.db.normalizeId("conversations", _id);
  if (conversationId === null) {
    return null;
  }
  const conversation = await ctx.db.get(conversationId);
  if (conversation === null) {
    return null;
  }
  const conversationMembers = await ctx.db
    .query("conversationMembers")
    .withIndex("by_conversation", (q) =>
      q.eq("conversationId", conversation._id),
    )
    .collect();
  const currentMember = conversationMembers.find((m) => m.userId === userId);
  if (currentMember === undefined) {
    console.log("not member of conversation", conversation._id);
    return null;
  }
  console.log("returning conversation", conversation._id);
  return await convertConversation(ctx, currentMember);
});

export const get = table.get;

export const by_priority = table.index(
  "by_priority",
  async function* (ctx, { key, inclusive, direction }) {
    const userId = await getCurrentUserIdOrThrow(ctx);
    const stream = streamQuery(ctx, {
      table: "conversationMembers",
      index: "by_latest_message_time",
      startIndexKey: [userId, ...key],
      startInclusive: inclusive,
      order: direction,
    });
    for await (const [conversation, _indexKey] of stream) {
      console.log("conversation", conversation.conversationId);
      yield conversation.conversationId;
    }
  },
);
