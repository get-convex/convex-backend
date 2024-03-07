import { query, mutation } from "./_generated/server";

export const setupBoatJoin = mutation(
  async ({ db }, { n, m }: { n: number; m: number }) => {
    // Clear tables in case we're reusing an existing database.
    for await (const boat of db.query("boats")) {
      await db.delete(boat._id);
    }
    for await (const boatVote of db.query("boatVotes")) {
      await db.delete(boatVote._id);
    }

    const boatIds = [];
    for (let i = 0; i < n; i++) {
      const boatId = await db.insert("boats", {});
      boatIds.push(boatId);
    }
    for (let i = 0; i < m; i++) {
      const boatId = boatIds[i % n];
      await db.insert("boatVotes", { boat: boatId });
    }
  },
);

export const queryBoatJoin = query(async ({ db }) => {
  let out = 0;
  const boats = await db.query("boats").collect();
  for (const boat of boats) {
    const votes = await db
      .query("boatVotes")
      .withIndex("by_boat", (q) => q.eq("boat", boat._id))
      .collect();
    out += votes.length;
  }
  return out;
});

export const setupUsers = mutation(async ({ db }, { n }: { n: number }) => {
  // Clear table in case we're reusing an existing database.
  for await (const user of db.query("users")) {
    await db.delete(user._id);
  }
  for (let i = 0; i < n; i++) {
    await db.insert("users", { identity: i });
  }
});

/// Simulating workload where we do a row-level-security check.
/// For each row fetched from db, we check it against the auth by re-fetching
/// the authenticated user.
/// Since this is a common pattern, we want it to be cacheable.
export const queryRepeatedAuth = query(async ({ db }, { n }: { n: number }) => {
  const myIdentity = 0; // in a real workload this would come from ctx.auth
  for (let i = 0; i < n; i++) {
    await db
      .query("users")
      .withIndex("by_identity", (q) => q.eq("identity", myIdentity))
      .first();
  }
});

/// Simulating workload where the query wants to return results in relevance
/// order according to product logic. So it first fetches the authenticated
/// user, then more recent users, then users with nearby IDs, then all users.
/// Then a real workload would dedupe and sort the results.
/// Thus users are fetched multiple times, so we can serve some of them from cache.
export const queryManyWays = query(async ({ db }, { n }: { n: number }) => {
  const myIdentity = n; // in a real workload this would come from ctx.auth
  const me = await db
    .query("users")
    .withIndex("by_identity", (q) => q.eq("identity", myIdentity))
    .first();
  if (!me) {
    throw new Error("no user");
  }
  const _moreRecent = await db
    .query("users")
    .withIndex("by_creation_time", (q) =>
      q.gte("_creationTime", me._creationTime),
    )
    .take(n);
  for (let i = 0; i < n; i++) {
    const _nearbyIds = await db
      .query("users")
      .withIndex("by_identity", (q) =>
        q.gt("identity", myIdentity - i).lt("identity", myIdentity + i),
      )
      .collect();
  }
  const _allUsers = await db.query("users").collect();
});
