import { Id } from "./_generated/dataModel";
import { mutation } from "./_generated/server";
import { api } from "./_generated/api";

// @snippet start self-destructing-message
function formatMessage(body: string, secondsLeft: number) {
  return `${body} (This message will self-destruct in ${secondsLeft} seconds)`;
}

export default mutation(
  async (
    { db, scheduler },
    { body, author }: { body: string; author: string },
  ) => {
    const id = await db.insert("messages", {
      body: formatMessage(body, 5),
      author,
    });
    await scheduler.runAfter(1000, api.sendExpiringMessage.update, {
      messageId: id,
      body,
      secondsLeft: 4,
    });
  },
);

export const update = mutation(
  async (
    { db, scheduler },
    {
      messageId,
      body,
      secondsLeft,
    }: { messageId: Id<"messages">; body: string; secondsLeft: number },
  ) => {
    if (secondsLeft > 0) {
      await db.patch(messageId, { body: formatMessage(body, secondsLeft) });
      await scheduler.runAfter(1000, api.sendExpiringMessage.update, {
        messageId,
        body,
        secondsLeft: secondsLeft - 1,
      });
    } else {
      await db.delete(messageId);
    }
  },
);
// @snippet end self-destructing-message
