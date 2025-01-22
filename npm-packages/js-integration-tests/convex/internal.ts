import { internal } from "./_generated/api";
import { Id } from "./_generated/dataModel";
import { mutation, internalMutation, query } from "./_generated/server";

function formatMessage(body: string, secondsLeft: number) {
  return `${body} (This message will self-destruct in ${secondsLeft} seconds)`;
}

export const getMessage = query(
  async ({ db }, { messageId }: { messageId: Id<"messages"> }) => {
    return await db.get(messageId);
  },
);

export const sendMessage = mutation(
  async ({ db }, { body, channel }: { body: string; channel: string }) => {
    const id = await db.insert("messages", {
      text: body,
      channel: channel,
    });
    return id;
  },
);

export const sendExpiringMessage = mutation(
  async (
    { db, scheduler },
    { body, channel }: { body: string; channel: string },
  ) => {
    const id = await db.insert("messages", {
      text: formatMessage(body, 1),
      channel: channel,
    });
    await scheduler.runAfter(1000, internal.internal.update, {
      messageId: id,
      body,
      secondsLeft: 0,
    });
    return id;
  },
);

export const update = internalMutation(
  async (
    { db, scheduler },
    {
      messageId,
      body,
      secondsLeft,
    }: {
      messageId: Id<"messages">;
      body: string;
      secondsLeft: number;
    },
  ) => {
    if (secondsLeft > 0) {
      await db.patch(messageId, { text: formatMessage(body, secondsLeft) });
      await scheduler.runAfter(1000, internal.internal.update, {
        messageId,
        body,
        secondsLeft: secondsLeft - 1,
      });
    } else {
      await db.delete(messageId);
    }
  },
);
