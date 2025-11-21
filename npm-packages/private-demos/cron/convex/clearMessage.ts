import { mutation } from "./_generated/server";
import { api } from "./_generated/api";

export default mutation({
  handler: async ({ db }, { n }: { n: number }) => {
    for (const message of await db.query("messages").take(n)) {
      await db.delete(message._id);
    }
  },
});

export const scheduleClearData = mutation({
  handler: async ({ scheduler }, { n = 10 }: { n?: number }) => {
    await scheduler.runAfter(n * 1000, api.clearMessage.default, { n: 1 });
  },
});

export const doNothing = mutation({
  args: {},
  handler: () => {
    // intentional noop.
  },
});
