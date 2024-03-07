import { Doc } from "../../_generated/dataModel";
import { queryPrivateSystem } from "../secretSystemTables";

export const deploymentState = queryPrivateSystem({
  args: {},
  handler: async function ({ db }): Promise<Doc<"_backend_state">> {
    return (await db.query("_backend_state").first())!;
  },
});
