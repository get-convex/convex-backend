import { s, streamQuery } from "./schema";

const t = s.table("users", async (ctx, _id) => {
  const normalizedId = ctx.db.normalizeId("users", _id);
  if (normalizedId === null) {
    return null;
  }
  const user = await ctx.db.get(normalizedId);
  if (user === null) {
    return null;
  }
  return { ...user, name: user.email ?? user.name ?? "Unknown" };
});

export const get = t.get;

export const by_id = t.index(
  "by_id",
  async function* (ctx, { key, inclusive, direction }) {
    const stream = streamQuery(ctx, {
      table: "users",
      index: "by_id",
      startIndexKey: key as any[],
      startInclusive: inclusive,
      order: direction,
    });
    for await (const [user, _indexKey] of stream) {
      yield user._id;
    }
  },
);
