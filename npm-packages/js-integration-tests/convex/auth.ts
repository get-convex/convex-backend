import { api } from "./_generated/api";
import { query, mutation } from "./_generated/server";

export const q = query(async ({ auth }) => {
  return await auth.getUserIdentity();
});

export const m = mutation(async ({ auth }) => {
  return await auth.getUserIdentity();
});

export const s = mutation(async ({ scheduler, auth }) => {
  if (!auth.getUserIdentity()) {
    throw new Error("not authed");
  }
  await scheduler.runAfter(0, api.actions.auth.storeUser);
});
