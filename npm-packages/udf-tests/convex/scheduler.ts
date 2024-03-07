import { makeFunctionReference, queryGeneric } from "convex/server";
import { api } from "./_generated/api";
import { mutation } from "./_generated/server";

export const getScheduledJobs = queryGeneric(async ({ db }) => {
  return await db.system.query("_scheduled_functions").collect();
});

export const scheduleWithArbitraryJson = mutation(async ({ scheduler }) => {
  // We should be able to schedule with arbitrary json arguments.
  // Scheduling this should succeed, even though actually executing the insert
  // would fail.
  await scheduler.runAfter(1000, api.basic.insertObject, {
    _id: "This is random ID",
    _string: "This is random body",
    _array: ["1", 8, 29902],
    _object: { a: "one", b: "two", c: "three" },
  });
});

export const scheduleAfter = mutation(
  async ({ scheduler }, { delayMs }: { delayMs: number }) => {
    await scheduler.runAfter(delayMs, api.basic.insertObject, {});
  },
);

export const scheduleAtTimestamp = mutation(
  async ({ scheduler }, { ts }: { ts: number }) => {
    await scheduler.runAt(ts, api.basic.insertObject, {});
  },
);

// Argument is still timestamp but we convert to Date() before calling invokeAt.
export const scheduleAtDate = mutation(
  async ({ scheduler }, { ts }: { ts: number }) => {
    await scheduler.runAt(new Date(ts), api.basic.insertObject, {});
  },
);

// Argument is still timestamp but we convert to Date() before calling invokeAt.
export const scheduleByName = mutation(
  async ({ scheduler }, { udfPath }: { udfPath: string }) => {
    const functionReference = makeFunctionReference<"mutation" | "action">(
      udfPath,
    );
    await scheduler.runAfter(1000, functionReference, {});
  },
);

export const scheduleMany = mutation(
  async ({ scheduler }, { limit, obj }: { limit: number; obj: any }) => {
    for (let i = 0; i < limit; i++) {
      await scheduler.runAfter(1000, api.basic.insertObject, obj);
    }
  },
);
