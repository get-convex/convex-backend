import { Id } from "./_generated/dataModel";
import { mutation } from "./_generated/server";

export const reportContentiously = mutation(
  async (
    { db },
    {
      x,
      y,
      ts,
      session,
    }: { x: number; y: number; ts: number; session: string },
  ): Promise<Id> => {
    let pos = await db
      .query("positions")
      .filter((q) => q.eq(q.field("session"), session))
      .first();
    if (pos === null) {
      pos = { session, x, y, clientSentTs: ts, serverSentTs: Date.now() };
      return db.insert("positions", pos);
    } else {
      await db.patch(pos._id, { x, y, ts, serverSentTs: Date.now() });
      return pos._id;
    }
  },
);

export const report = mutation(
  async (
    { db },
    {
      x,
      y,
      ts,
      session,
      id,
    }: {
      x: number;
      y: number;
      ts: number;
      session: string;
      id: Id | null;
    },
  ): Promise<Id> => {
    let pos = null;
    if (id !== null) {
      pos = await db.get(id);
    }
    if (pos === null) {
      pos = { session, x, y, clientSentTs: ts, serverSentTs: Date.now() };
      return await db.insert("positions", pos);
    } else {
      await db.patch(pos._id, { x, y, ts, serverSentTs: Date.now() });
      return pos._id;
    }
  },
);
