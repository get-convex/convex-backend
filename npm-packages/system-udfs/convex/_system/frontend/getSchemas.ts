import { DatabaseReader } from "../../_generated/server";
import { Doc } from "../../_generated/dataModel";
import { queryPrivateSystem } from "../secretSystemTables";
import { v } from "convex/values";

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
  args: { componentId: v.optional(v.union(v.string(), v.null())) },
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

export const schemaValidationProgress = queryPrivateSystem({
  args: { componentId: v.optional(v.union(v.string(), v.null())) },
  handler: async function ({
    db,
  }): Promise<{ numDocsValidated: number; totalDocs: number | null } | null> {
    const pending = await getSchemaByState(db, "pending");
    if (!pending) {
      return null;
    }
    const schemaValidationProgressDoc = await db
      .query("_schema_validation_progress")
      .withIndex("by_schema_id", (q) => q.eq("schemaId", pending._id))
      .unique();
    if (!schemaValidationProgressDoc) {
      return null;
    }
    return {
      numDocsValidated: Number(schemaValidationProgressDoc.numDocsValidated),
      totalDocs: schemaValidationProgressDoc.totalDocs
        ? Number(schemaValidationProgressDoc.totalDocs)
        : null,
    };
  },
});
