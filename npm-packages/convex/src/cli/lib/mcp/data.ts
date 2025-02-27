import { deploymentSelectionFromOptions } from "../api.js";
import { fetchDeploymentCredentialsProvisionProd } from "../api.js";
import { z } from "zod";
import { runSystemQuery } from "../run.js";
import { ConvexTool } from "./tool.js";
import { PaginationResult } from "../../../server/pagination.js";

const inputSchema = z.object({
  tableName: z.string().describe("The name of the table to read from."),
  order: z.enum(["asc", "desc"]).describe("The order to sort the results in."),
  cursor: z.string().optional().describe("The cursor to start reading from."),
  limit: z
    .number()
    .max(1000)
    .optional()
    .describe("The maximum number of results to return, defaults to 100."),
});

const outputSchema = z.object({
  page: z.array(z.any()),
  isDone: z.boolean(),
  continueCursor: z.string(),
});

const description = `
Read a page of data from a table in the project's Convex deployment.

Output:
- page: A page of results from the table.
- isDone: Whether there are more results to read.
- continueCursor: The cursor to use to read the next page of results.
`.trim();

export const DataTool: ConvexTool<typeof inputSchema, typeof outputSchema> = {
  name: "data",
  description,
  inputSchema,
  outputSchema,
  handler: async (ctx, args) => {
    const deploymentSelection = await deploymentSelectionFromOptions(
      ctx,
      ctx.cmdOptions,
    );
    const credentials = await fetchDeploymentCredentialsProvisionProd(
      ctx,
      deploymentSelection,
    );
    const paginationResult = (await runSystemQuery(ctx, {
      deploymentUrl: credentials.url,
      adminKey: credentials.adminKey,
      functionName: "_system/cli/tableData",
      componentPath: undefined,
      args: {
        table: args.tableName,
        order: args.order,
        paginationOpts: {
          numItems: args.limit ?? 100,
          cursor: args.cursor ?? null,
        },
      },
    })) as unknown as PaginationResult<any>;
    return {
      page: paginationResult.page,
      isDone: paginationResult.isDone,
      continueCursor: paginationResult.continueCursor,
    };
  },
};
