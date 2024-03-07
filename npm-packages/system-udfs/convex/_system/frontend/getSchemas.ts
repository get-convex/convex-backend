import { DatabaseReader } from "../../_generated/server";
import { Doc } from "../../_generated/dataModel";
import { queryPrivateSystem } from "../secretSystemTables";

type SchemaMetadata = Doc<"_schemas">;

export const getSchemaByState = async (
  db: DatabaseReader,
  state: SchemaMetadata["state"]["state"],
) =>
  await db
    .query("_schemas")
    .withIndex("by_state", (q) => q.eq("state", { state }))
    .unique();

export default queryPrivateSystem({
  args: {},
  handler: async function ({ db }): Promise<{
    active?: string;
    inProgress?: string;
  }> {
    const active = await getSchemaByState(db, "active");
    const pending = await getSchemaByState(db, "pending");
    const validated = await getSchemaByState(db, "validated");

    if (pending && validated) {
      throw new Error("Unexpectedly found both pending and validated schemas");
    }

    return {
      active: active?.schema,
      inProgress: pending?.schema || validated?.schema,
    };
  },
});
