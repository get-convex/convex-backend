import { query } from "./_generated/server";
import { api } from "./_generated/api";

export const getName = query(async function ({ auth }) {
  const user = await auth.getUserIdentity();
  if (user !== null) {
    return user.name ?? "No name provided";
  }
  return null;
});

export const getIdentifier = query(async function ({ auth }) {
  const user = await auth.getUserIdentity();
  if (user !== null) {
    return user.tokenIdentifier;
  }
  return null;
});

// If "objects" is empty, returns the current time without reading `ctx.auth`.
// Then the query can be cached across users.
// If "objects" is not empty, returns the auth token identifier, which cannot
// be cached across users.
export const conditionallyCheckAuth = query(async function (ctx) {
  const objects = await ctx.db.query("objects").collect();
  if (objects.length === 0) {
    return new Date().toString();
  }
  const user = await ctx.auth.getUserIdentity();
  return user?.tokenIdentifier ?? "No user";
});

export const conditionallyCheckAuthInSubquery = query(
  async function (ctx): Promise<string> {
    return await ctx.runQuery(api.auth.conditionallyCheckAuth);
  },
);

export const getUserIdentityDebug = query(async function ({ auth }) {
  return await auth.getUserIdentityDebug();
});

export const getUserIdentityInsecure = query(async function ({ auth }) {
  return await auth.getUserIdentityInsecure();
});

export const getIdentityType = query(async function ({ auth }) {
  // This is a helper function to identify what type of identity we have
  // for testing purposes
  const identity = await auth.getUserIdentity();
  const insecureToken = await auth.getUserIdentityInsecure();
  
  if (insecureToken !== null) {
    return "PlaintextUser";
  } else if (identity !== null) {
    return "User";
  } else {
    return "Unknown";
  }
});

export const testAdminAccess = query(async function ({ auth }) {
  // Check if we have an insecure token (PlaintextUser case)
  const insecureToken = await auth.getUserIdentityInsecure();
  
  if (insecureToken !== null) {
    // PlaintextUser identities should NOT have admin access
    throw new Error("BadDeployKey: PlaintextUser identities cannot access admin functions");
  }
  
  // For all other identity types (regular users, admin, system, etc.), allow access
  // In a real app, this would check for actual admin permissions
  return { hasAdminAccess: true };
});
