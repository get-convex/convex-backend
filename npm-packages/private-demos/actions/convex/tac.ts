import { api } from "./_generated/api";
import { Id } from "./_generated/dataModel";
import { action } from "./_generated/server";

export const tac = action(
  async ({ runMutation }, { author }: { author: string }) => {
    await runMutation(api.sendMessage.default, {
      format: "text",
      body: "tac",
      author,
    });
  },
);

// Note that this does not need a action but we are using one to test scheduling
// to and from actions.
export default action(
  async (
    { runAction, scheduler },
    { author }: { author: string },
  ): Promise<Id<"_scheduled_functions">> => {
    // Call another action, that then calls a mutation to write the 'tac.
    await runAction(api.tac.tac, { author });

    // Schedule an action in node to write the 'toe'.
    return await scheduler.runAfter(1000, api.toe.default, { author });
  },
);
