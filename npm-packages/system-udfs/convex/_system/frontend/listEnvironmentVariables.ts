import { Doc } from "../../_generated/dataModel";
import { queryPrivateSystem, requireOperation } from "../secretSystemTables";
export default queryPrivateSystem({
  args: {},
  handler: async ({ db }): Promise<Doc<"_environment_variables">[]> => {
    requireOperation("ViewEnvironmentVariables");
    return await db
      .query("_environment_variables")
      .withIndex("by_name")
      .order("asc")
      .collect();
  },
});
