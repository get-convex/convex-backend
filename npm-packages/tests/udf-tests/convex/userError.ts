import { internal } from "./_generated/api";
import { Doc, Id } from "./_generated/dataModel";
import { internalMutation, mutation, query } from "./_generated/server";

/* eslint-disable */
function _aPrivateFunction() {}

export function somethingElse() {}

export const badArgumentsError = query(async ({ db }) => {
  try {
    // @ts-expect-error
    await db.get(null);
  } catch (error: any) {
    return error.toString();
  }
  throw new Error("Unexpected success for invalid arguments");
});

export const badIdError = query(async ({ db }) => {
  try {
    await db.get("abc" as any);
  } catch (error: any) {
    return error.toString();
  }
  throw new Error("Unexpected success for invalid Id");
});

export const insertErrorWithBigint = mutation(async ({ db }) => {
  try {
    await db.insert("table", { a: BigInt("123"), bad: [undefined] });
  } catch (error: any) {
    return error.toString();
  }
});

export const insertError = mutation(async ({ db }) => {
  try {
    // @ts-expect-error
    await db.insert("_notallowed", {});
  } catch (error: any) {
    return error.toString();
  }
});

export const patchError = mutation(async ({ db }) => {
  const id = await db.insert("ok", {});
  await db.delete(id);
  try {
    // @ts-expect-error
    await db.patch(id, { not: "ok" });
  } catch (error: any) {
    return error.toString();
  }
});

export const patchValueNotAnObject = mutation(async ({ db }) => {
  const id = await db.insert("ok", {});
  try {
    // @ts-expect-error
    await db.patch(id, "ok");
  } catch (error: any) {
    return error.toString();
  }
});

export const replaceError = mutation(async ({ db }) => {
  const id = await db.insert("ok", {});
  await db.delete(id);
  try {
    // @ts-expect-error
    await db.replace(id, { not: "ok" });
  } catch (error: any) {
    return error.toString();
  }
});

export const deleteError = mutation(async ({ db }) => {
  const id = await db.insert("ok", {});
  await db.delete(id);
  try {
    await db.delete(id);
  } catch (error: any) {
    return error.toString();
  }
});

export const syscallError = mutation(async ({ db }) => {
  // Try inserting a document that already has its ID field specified.
  await db.insert("table", { _id: 1729 });
});

export const nonexistentTable = mutation(
  async ({ db }, args: { nonexistentId: string }) => {
    const fakeId = args.nonexistentId as Id<any>;
    const assertMissing = async (thunk: () => Promise<void>) => {
      try {
        await thunk();
      } catch (error: any) {
        if (!error.toString().includes(`Table for ID "${fakeId}" not found`)) {
          throw error;
        }
        return;
      }
      throw new Error("expected TableNotFound error");
    };
    if ((await db.get(fakeId)) !== null) {
      throw new Error("expected db.get to return null");
    }
    await assertMissing(async () => {
      await db.delete(fakeId);
    });
    await assertMissing(async () => {
      await db.patch(fakeId, { f: 0 });
    });
    await assertMissing(async () => {
      await db.replace(fakeId, { f: 0 });
    });
    let results: Doc<any>[] = await db
      .query("ok")
      .filter((q) => q.eq(q.field("_id"), fakeId))
      .collect();
    if (results.length !== 0) {
      throw new Error(
        "expected query filtering to fake ID to return 0 results",
      );
    }
    results = await db
      .query("boatVotes")
      .withIndex("by_boat", (q) => q.gt("boat", fakeId))
      .collect();
    if (results.length !== 0) {
      throw new Error(
        "expected query filtering to fake ID to return 0 results",
      );
    }
  },
);

export const indexOnNonexistentTable = mutation(async ({ db }) => {
  // Querying a non-existent table by_creation_time is fine.
  let results = await db.query("missing" as any).collect();
  if (results.length !== 0) {
    throw new Error('expected db.query("missing") to return 0 results');
  }

  // Querying a non-existent table with filters is fine too
  results = await db
    .query("missing" as any)
    .filter((q) => q.eq(q.field("foo"), "foo"))
    .collect();
  if (results.length !== 0) {
    throw new Error('expected db.query("missing") to return 0 results');
  }

  // Querying a non-existent table with index throws error about the index
  // missing (not the table missing).
  const assertIndexNotFound = async (
    thunk: () => Promise<void>,
    indexName: string,
  ) => {
    try {
      await thunk();
    } catch (error: any) {
      if (!error.toString().includes(`Index ${indexName} not found`)) {
        throw error;
      }
      return;
    }
    throw new Error("expected TableNotFound error");
  };
  await assertIndexNotFound(async () => {
    await db
      .query("missing" as any)
      .withIndex("by_foo")
      .collect();
  }, "missing.by_foo");

  // Same error with search index
  await assertIndexNotFound(async () => {
    await db
      .query("missing" as any)
      .withSearchIndex("search_foo", (q) => q.search("foo", "foo"))
      .collect();
  }, "missing.search_foo");
});

export const nonexistentId = mutation({
  handler: async (
    { db },
    {
      nonexistentSystemId,
      nonexistentUserId,
    }: { nonexistentSystemId: string; nonexistentUserId: string },
  ) => {
    // Try performing operations on Ids where the table exists but the Id doesn't.
    const fakeSystemId = nonexistentSystemId as Id<any>;
    const fakeUserId = nonexistentUserId as Id<any>;
    if ((await db.get(fakeUserId)) !== null) {
      throw new Error("expected db.get to return null");
    }
    if ((await db.system.get(fakeSystemId)) !== null) {
      throw new Error("expected db.system.get to return null");
    }
  },
});

export const nonexistentSystemIdFails = mutation({
  handler: async (
    { db },
    { nonexistentSystemId }: { nonexistentSystemId: string },
  ) => {
    // Try performing operations on Ids where the table exists but the Id doesn't.
    const fakeSystemId = nonexistentSystemId as Id<any>;
    await db.get(fakeSystemId);
  },
});

export const nonexistentUserIdFails = mutation({
  handler: async (
    { db },
    { nonexistentUserId }: { nonexistentUserId: string },
  ) => {
    // Try performing operations on Ids where the table exists but the Id doesn't.
    const fakeUserId = nonexistentUserId as Id<any>;
    await db.system.get(fakeUserId);
  },
});

export const incorrectExplicitIdGet = mutation(async ({ db }) => {
  const id = await db.insert("objects", {});
  await db.get("table", id as Id<any>);
});

export const incorrectExplicitIdGetSystem = mutation(async (ctx) => {
  const systemId = await ctx.scheduler.runAfter(
    0,
    internal.userError.doNothing,
    {},
  );
  await ctx.db.system.get("_storage", systemId as Id<any>);
});

export const doNothing = internalMutation(async () => {});

export const incorrectExplicitIdPatch = mutation(async ({ db }) => {
  const id = await db.insert("objects", {});
  await db.patch("table", id as Id<any>, {});
});

export const incorrectExplicitIdReplace = mutation(async ({ db }) => {
  const id = await db.insert("objects", {});
  await db.replace("table", id as Id<any>, {});
});

export const incorrectExplicitIdDelete = mutation(async ({ db }) => {
  const id = await db.insert("objects", {});
  await db.delete("table", id as Id<any>);
});

export const privateSystemQuery = query(
  async ({ db }, { tableName }: { tableName: any }) => {
    return await db.system.query(tableName).collect();
  },
);

export const privateSystemGet = query(
  async ({ db }, { id }: { id: Id<any> }) => {
    return await db.system.get(id);
  },
);

export const unhandledRejection = mutation(async ({ db }) => {
  const id = await db.insert("test", {});

  const p1 = db.get("abc" as Id<any>);
  const p2 = db.get(id);

  // Await the second (successful) promise without attaching a handler
  // to the first promise. The system is forced to execute the first
  // promise first, which will then fail without a handler.
  return await p2;
});

async function asyncException(awaitFirst = false) {
  if (awaitFirst) {
    // code after this await runs asynchronously
    await Promise.resolve();
  }
  throw new Error("This is a custom exception");
}

export const asyncExceptionBeforeAwait = mutation(async ({ db }) => {
  try {
    await asyncException(false);
  } catch (e: any) {
    return e.toString();
  }
});

export const asyncExceptionAfterAwait = mutation(async ({ db }) => {
  try {
    await asyncException(true);
  } catch (e: any) {
    return e.toString();
  }
});

export const throwString = mutation(async ({ db }) => {
  try {
    throw "a string";
  } catch (e: any) {
    return `${typeof e} - ${e}`;
  }
});
