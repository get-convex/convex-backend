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

export default queryPrivateSystem("ViewData")({
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

export const schemaValidationProgress = queryPrivateSystem("ViewData")({
  args: { componentId: v.optional(v.union(v.string(), v.null())) },
  handler: async function ({
    db,
  }): Promise<{ numDocsValidated: number; totalDocs: number | null } | null> {
    const pending = await getSchemaByState(db, "pending");
    if (!pending) {
      return null;
    }
    const attempts = await db
      .query("_schema_validations")
      .withIndex("by_schema_id_and_table_name", (q) =>
        q.eq("schemaId", pending._id),
      )
      .collect();
    const rows = await Promise.all(
      attempts.map((attempt) => validationProgress(db, attempt)),
    );
    if (rows.length === 0) {
      const legacy = await db
        .query("_schema_validation_progress")
        .withIndex("by_schema_id", (q) => q.eq("schemaId", pending._id))
        .unique();
      return legacy === null
        ? null
        : {
            numDocsValidated: Number(legacy.numDocsValidated),
            totalDocs:
              legacy.totalDocs === null
                ? null
                : Number(legacy.totalDocs) || null,
          };
    }
    return {
      numDocsValidated: rows.reduce(
        (sum, row) => sum + Number(row.numDocsValidated),
        0,
      ),
      totalDocs: rows.every((row) => row.totalDocs !== null)
        ? rows.reduce((sum, row) => sum + Number(row.totalDocs), 0) || null
        : null,
    };
  },
});

async function validationProgress(
  db: DatabaseReader,
  attempt: Doc<"_schema_validations">,
) {
  const progress = await db
    .query("_schema_validation_progress")
    .withIndex("by_validation_id", (q) => q.eq("validationId", attempt._id))
    .unique();
  return {
    ...attempt,
    numDocsValidated: progress?.numDocsValidated ?? BigInt(0),
    totalDocs: progress?.totalDocs ?? null,
  };
}
