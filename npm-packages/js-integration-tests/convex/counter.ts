import { Id } from "./_generated/dataModel";
import { mutation, query } from "./_generated/server";

export const get = query(
  async ({ db }, { counter }: { counter: Id<"counters"> }) => {
    const doc = await db.get(counter);
    if (!doc) throw new Error("no counter found");
    return doc.count;
  },
);

export const create = mutation(async ({ db }) => {
  return await db.insert("counters", { count: 0 });
});

export const increment = mutation(
  async ({ db }, { counter }: { counter: Id<"counters"> }) => {
    const doc = await db.get(counter);
    if (!doc) throw new Error("no counter found");
    return await db.replace(counter, { count: doc.count + 1 });
  },
);
