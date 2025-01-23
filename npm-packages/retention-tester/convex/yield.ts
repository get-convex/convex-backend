import {
  DatabaseReader,
  DatabaseWriter,
  internalMutation,
} from "./_generated/server";

export async function didConvexYield(db: DatabaseReader) {
  return (await db.query("yield").unique())?.doYouYield ?? false;
}

export const convexYields = internalMutation(async ({ db }) => {
  await setConvexYields(db, true);
});

export const convexIsReadyToRumble = internalMutation(async ({ db }) => {
  await setConvexYields(db, false);
});

async function setConvexYields(db: DatabaseWriter, isYielding: boolean) {
  const value = await db.query("yield").unique();
  if (value) {
    await db.delete(value._id);
  }
  await db.insert("yield", { doYouYield: isYielding });
}
