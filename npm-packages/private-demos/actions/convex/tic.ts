import { api } from "./_generated/api";
import { Id } from "./_generated/dataModel";
import { mutation } from "./_generated/server";

export default mutation(
  async (
    { db, scheduler },
    { author }: { author: string },
  ): Promise<Id<"_scheduled_functions">> => {
    const message = {
      format: "text" as const,
      body: "tic",
      author,
    };
    await db.insert("messages", message);
    return await scheduler.runAfter(1000, api.tac.default, { author });
  },
);
