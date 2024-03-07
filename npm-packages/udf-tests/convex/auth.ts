import { query } from "./_generated/server";

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
