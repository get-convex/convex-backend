import { action, mutation, query } from "./_generated/server";

export const insertValue = mutation(async ({ db }) => {
  await db.insert("sets", { set: new Set([1, 2]) });
});

export const listValues = query(async () => {
  return new Set();
});

export const mutationReturningSet = mutation(async () => {
  return new Set(["hello", "world"]);
});

export const actionReturningSet = action(async () => {
  return new Set(["hello", "world"]);
});

export const queryWithAnyArg = query(async (_, _args: { x: any }) => {
  // Do nada
});
export const mutationWithAnyArg = mutation(async (_, _args: { x: any }) => {
  // Do nada
});
export const actionWithAnyArg = action(async (_, _args: { x: any }) => {
  // Do nada
});
