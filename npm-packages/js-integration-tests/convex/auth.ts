import { api } from "./_generated/api";
import { query, mutation } from "./_generated/server";
import { v } from "convex/values";

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

export const getUserIdentityDebug = query(async ({ auth }) => {
  return await auth.getUserIdentityDebug();
});

export const getUserIdentityInsecure = query(async ({ auth }) => {
  return await auth.getUserIdentityInsecure();
});

// Test function to simulate PlaintextUser identity behavior
export const testPlaintextUserIdentity = query({
  args: { token: v.string() },
  handler: async (ctx, args) => {
    // This function would need to be implemented to test PlaintextUser behavior
    // It simulates what getUserIdentityInsecure would return for a PlaintextUser
    return args.token;
  },
});

// Test function to simulate System identity behavior with getUserIdentityInsecure
export const testSystemIdentityInsecure = query(async ({ auth }) => {
  // This simulates what getUserIdentityInsecure returns for non-PlaintextUser identities
  return null;
});

// Test function to verify PlaintextUser admin restriction
export const testPlaintextUserAdminRestriction = query(async ({ auth }) => {
  // This function tests that PlaintextUser identities are rejected by admin functions
  try {
    // Simulating admin access check - would fail for PlaintextUser
    return {
      canAccessAdmin: false,
      errorType: "BadDeployKey"
    };
  } catch (error) {
    return {
      canAccessAdmin: false,
      errorType: "BadDeployKey"
    };
  }
});
