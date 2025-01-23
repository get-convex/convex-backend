import { action, query, mutation } from "./_generated/server";

export const createMap = mutation(async ({ db }) => {
  return await db.insert("maps", { map: new Map([["n", "m"]]) });
});

export const listValues = query(async () => {
  return new Map();
});

export const mutationReturningMap = mutation(async () => {
  return new Map([["key", "value"]]);
});

export const actionReturningMap = action(async () => {
  return new Map([["key", "value"]]);
});
