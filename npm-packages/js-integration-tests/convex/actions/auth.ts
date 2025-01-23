"use node";

import { UserIdentity } from "convex/server";
import { action } from "../_generated/server";
import { api } from "../_generated/api";

export const q = action(
  async ({ runQuery, auth }): Promise<UserIdentity | null> => {
    if (!auth.getUserIdentity()) {
      throw new Error("not authed");
    }
    return await runQuery(api.auth.q);
  },
);
export const m = action(
  async ({ runMutation, auth }): Promise<UserIdentity | null> => {
    if (!auth.getUserIdentity()) {
      throw new Error("not authed");
    }
    return await runMutation(api.auth.m);
  },
);

export const s = action(async ({ scheduler, auth }) => {
  if (!auth.getUserIdentity()) {
    throw new Error("not authed");
  }
  await scheduler.runAfter(0, api.actions.auth.storeUser);
});

export const storeUser = action(async ({ auth, runMutation }) => {
  const user = await auth.getUserIdentity();
  // Was the authentication information passed through to this action?
  await runMutation(api.storeObject.default, { foundUser: user !== null });
});
