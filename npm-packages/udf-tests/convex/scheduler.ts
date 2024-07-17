import { makeFunctionReference, queryGeneric } from "convex/server";
import { v } from "convex/values";
import { api } from "./_generated/api";
import { action, DatabaseReader, mutation, query } from "./_generated/server";

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

// Get the job id for the current mutation.
const getScheduledJobId = async (db: DatabaseReader) => {
  let jobId = null;
  for await (const job of db.system.query("_scheduled_functions")) {
    if (job.state.kind === "inProgress") {
      if (jobId !== null) {
        throw new Error("Multiple `inProgress` job ids");
      }
      jobId = job._id;
    }
  }
  return jobId;
};

// Find the current job_id and insert it to completed_job_ids.
export const insertMyJobId = mutation(async ({ db }) => {
  const jobId = await getScheduledJobId(db);
  if (jobId === null) {
    throw new Error("Failed to find jobId");
  }
  await db.insert("completedScheduledJobs", { jobId: jobId });
});

export const getJobById = query({
  args: { jobId: v.id("_scheduled_functions") },
  handler: async ({ db }, { jobId }) => {
    return await db.system.get(jobId);
  },
});

export const scheduleByString = action({
  args: {},
  handler: async (ctx): Promise<string> => {
    const jobId = await ctx.scheduler.runAfter(0, api.basic.insertObject, {});
    const job = (await ctx.runQuery(api.scheduler.getJobById, { jobId }))!;
    const jobPath = job.name;
    const jobFunction = makeFunctionReference<"mutation", any, any>(jobPath);
    await ctx.scheduler.runAfter(0, jobFunction, {});
    return jobPath;
  },
});
