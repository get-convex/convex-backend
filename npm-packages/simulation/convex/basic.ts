import { mutation, query } from "./_generated/server";

export const count = query(async ({ db }) => {
  let count = 0;
  for await (const _ of db.query("objects")) {
    count++;
  }
  return count;
});

export const insertObject = mutation(async ({ db }, obj) => {
  const id = await db.insert("objects", obj);
  return await db.get(id);
});
