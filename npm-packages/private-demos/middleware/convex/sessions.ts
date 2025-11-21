import { v } from "convex/values";
import { action, mutation } from "./_generated/server";
import {
  mutationWithSession,
  queryWithOptionalSession,
  withSession,
} from "./lib/withSession";
import { withReplacer } from "./lib/withReplacer";

/**
 * Creates a session and returns the id. For use with the SessionProvider on the
 * client.
 * Note: if you end up importing code from other modules that use sessions,
 * you'll likely want to move this code to avoid import cycles.
 */
export const create = mutation({
  args: {},
  handler: async ({ db }) => {
    return db.insert("sessions", {
      // TODO: insert your default values here
    });
  },
});

export const simpleMutation = mutationWithSession(async ({ db, session }) => {
  console.log("session:", session);
  console.log(db);
  return "hello";
});

// this only works with
export const mutationWithArg = mutationWithSession({
  args: { a: v.string() },
  handler: async ({ db, session }, { a }: { a: string }) => {
    console.log("session:", session);
    console.log("argument", a);
    console.log("db", db);
    return "hello";
  },
});

export const unvalidatedQueryNoArgNoObject = queryWithOptionalSession(
  async ({ db, session }) => {
    console.log(db, session);
    return "ehllo";
  },
);

export const unvalidatedQueryWithArgNoObject = queryWithOptionalSession(
  async ({ db, session }, { a }: { a: number }) => {
    console.log(db, session, a);
    return "something";
  },
);

export const unvalidatedQueryNoArgWithObject = queryWithOptionalSession({
  handler: async ({ db, session }) => {
    console.log(db, session);
    return "something";
  },
});

export const unvalidatedQueryWithArgWithObject = queryWithOptionalSession({
  args: {
    a: v.number(),
  },
  handler: async ({ db, session }, { a }) => {
    console.log(db, session);
    return a + 2;
  },
});

export const myMutationWithSession = mutationWithSession({
  args: {
    a: v.number(),
  },
  handler: async ({ db, session }, { a }) => {
    console.log(db, session, a);
    return "something";
  },
});

export const actionWithSession = action(
  // @ts-expect-error Should not compile because we need a DB
  withSession({
    handler: async () => {
      return "hello";
    },
  }),
);

// Composed middleware
export const mutationWithSessionAndReplacer = mutation(
  withSession(
    withReplacer({
      args: { a: v.number() },
      handler: async ({ db }, { a }) => {
        console.log(db, a);
        return Promise.resolve("hello world ".repeat(a));
      },
    }),
  ),
);
