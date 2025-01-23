"use node";

import { TableNamesInDataModel } from "convex/server";
import { api } from "../_generated/api";
import { action } from "../_generated/server";
import randomWords from "random-words";
import { DataModel } from "../_generated/dataModel";
import { rand } from "../common";

export default action(async ({ runAction, runMutation }) => {
  // Exercise an action involving an external package
  await runAction(api.actions.externalNodeDeps.encode, {
    str: randomWords({ exactly: 10 }).join(","),
  });
  const table: TableNamesInDataModel<DataModel> = "messages_with_search";
  const insertArgs = {
    channel: "global",
    timestamp: Date.now(),
    rand: rand(),
    ballastCount: 0,
    count: 1,
    table,
  };
  // Do an insert by using an action.
  await runMutation(api.insert.insertMessageWithArgs, insertArgs);
});
