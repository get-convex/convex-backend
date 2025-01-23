import { api } from "./_generated/api";
import { Id } from "./_generated/dataModel";
import { mutation, action } from "./_generated/server";

export const cancelJob = mutation({
  handler: async (
    { scheduler },
    { id }: { id: Id<"_scheduled_functions"> },
  ) => {
    await scheduler.cancel(id);
  },
});

export const tic = mutation({
  handler: async ({ scheduler }): Promise<Id<"_scheduled_functions">> => {
    return await scheduler.runAfter(0, api.cancelJob.tac);
  },
});

export const tac = action({
  handler: async ({ scheduler, runAction }) => {
    await sleep(2000);
    await scheduler.runAfter(0, api.cancelJob.toe);
    await runAction(api.simple.userTimeout);
  },
});

export const toe = mutation({
  handler: async () => {},
});

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
