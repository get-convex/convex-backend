import { z } from "zod";
import { ConvexTool } from "./tool.js";
import {
  deploymentSelectionFromOptions,
  fetchDeploymentCredentialsProvisionProd,
} from "../api.js";
import { runSystemQuery } from "../run.js";
import { deploymentFetch } from "../utils/utils.js";

const inputSchema = z.object({});

const outputSchema = z.object({
  tables: z.record(
    z.string(),
    z.object({
      schema: z.any().optional(),
      inferredSchema: z.any().optional(),
    }),
  ),
});

export const TablesTool: ConvexTool<typeof inputSchema, typeof outputSchema> = {
  name: "tables",
  description:
    "List all tables in the project's Convex deployment and their inferred and declared schema.",
  inputSchema,
  outputSchema,
  handler: async (ctx) => {
    const deploymentSelection = await deploymentSelectionFromOptions(
      ctx,
      ctx.cmdOptions,
    );
    const credentials = await fetchDeploymentCredentialsProvisionProd(
      ctx,
      deploymentSelection,
    );
    const schemaResponse: any = await runSystemQuery(ctx, {
      deploymentUrl: credentials.url,
      adminKey: credentials.adminKey,
      functionName: "_system/frontend/getSchemas",
      componentPath: undefined,
      args: {},
    });
    const schema: Record<string, z.infer<typeof activeSchemaEntry>> = {};
    if (schemaResponse.active) {
      const parsed = activeSchema.parse(JSON.parse(schemaResponse.active));
      for (const table of parsed.tables) {
        schema[table.tableName] = table;
      }
    }
    const fetch = deploymentFetch(ctx, {
      deploymentUrl: credentials.url,
      adminKey: credentials.adminKey,
    });
    const response = await fetch("/api/shapes2", {});
    const shapesResult: Record<string, any> = await response.json();

    const allTablesSet = new Set([
      ...Object.keys(shapesResult),
      ...Object.keys(schema),
    ]);
    const allTables = Array.from(allTablesSet);
    allTables.sort();

    const result: z.infer<typeof outputSchema>["tables"] = {};
    for (const table of allTables) {
      result[table] = {
        schema: schema[table],
        inferredSchema: shapesResult[table],
      };
    }
    return { tables: result };
  },
};

const activeSchemaEntry = z.object({
  tableName: z.string(),
  indexes: z.array(z.any()),
  searchIndexes: z.array(z.any()),
  vectorIndexes: z.array(z.any()),
  documentType: z.any(),
});

const activeSchema = z.object({ tables: z.array(activeSchemaEntry) });
