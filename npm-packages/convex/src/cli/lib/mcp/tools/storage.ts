import { z } from "zod";
import { ConvexTool } from "./index.js";
import { loadSelectedDeploymentCredentials } from "../../api.js";
import { runSystemQuery } from "../../run.js";
import { ConvexHttpClient } from "../../../../browser/index.js";
import { DefaultLogger } from "../../../../browser/logging.js";
import { getMcpDeploymentSelection } from "../requestContext.js";

// --- Storage List ---

const listInputSchema = z.object({
  deploymentSelector: z
    .string()
    .describe("Deployment selector (from the status tool)."),
  limit: z
    .number()
    .max(100)
    .optional()
    .describe("Maximum number of files to return. Defaults to 50."),
});

const listOutputSchema = z.object({
  files: z.array(
    z.object({
      storageId: z.string(),
      size: z.number().optional(),
      contentType: z.string().optional(),
    }),
  ),
});

export const StorageListTool: ConvexTool<
  typeof listInputSchema,
  typeof listOutputSchema
> = {
  name: "storage_list",
  description:
    "List files stored in the Convex file storage for a deployment. Returns storage IDs, sizes, and content types.",
  inputSchema: listInputSchema,
  outputSchema: listOutputSchema,
  handler: async (ctx, args) => {
    const { projectDir, deployment } =
      await ctx.decodeDeploymentSelectorReadOnly(args.deploymentSelector);
    process.chdir(projectDir);
    const deploymentSelection = await getMcpDeploymentSelection(
      ctx,
      deployment,
    );
    const credentials = await loadSelectedDeploymentCredentials(
      ctx,
      deploymentSelection,
    );
    const result = (await runSystemQuery(ctx, {
      deploymentUrl: credentials.url,
      adminKey: credentials.adminKey,
      functionName: "_system/cli/tableData",
      componentPath: undefined,
      args: {
        table: "_storage",
        order: "desc",
        paginationOpts: {
          numItems: args.limit ?? 50,
          cursor: null,
        },
      },
    })) as any;

    const files = (result?.page ?? []).map((doc: any) => ({
      storageId: doc._id,
      size: doc.size,
      contentType: doc.contentType,
    }));

    return { files };
  },
};

// --- Storage Get URL ---

const getUrlInputSchema = z.object({
  deploymentSelector: z
    .string()
    .describe("Deployment selector (from the status tool)."),
  storageId: z.string().describe("The storage ID of the file to get a URL for."),
});

const getUrlOutputSchema = z.object({
  url: z.string().nullable().describe("The serving URL for the file, or null if not found."),
});

export const StorageGetUrlTool: ConvexTool<
  typeof getUrlInputSchema,
  typeof getUrlOutputSchema
> = {
  name: "storage_get_url",
  description:
    "Get a serving URL for a file in Convex file storage by its storage ID.",
  inputSchema: getUrlInputSchema,
  outputSchema: getUrlOutputSchema,
  handler: async (ctx, args) => {
    const { projectDir, deployment } =
      await ctx.decodeDeploymentSelectorReadOnly(args.deploymentSelector);
    process.chdir(projectDir);
    const metadata = await getMcpDeploymentSelection(ctx, deployment);
    const credentials = await loadSelectedDeploymentCredentials(ctx, metadata);
    const logger = new DefaultLogger({ verbose: false });
    const client = new ConvexHttpClient(credentials.url, { logger });
    client.setAdminAuth(credentials.adminKey);

    try {
      const url = await client.query(
        "_system/cli/storageGetUrl" as any,
        { storageId: args.storageId } as any,
      );
      return { url: url ? String(url) : null };
    } catch {
      return { url: null };
    }
  },
};

// --- Storage Delete ---

const deleteInputSchema = z.object({
  deploymentSelector: z
    .string()
    .describe("Deployment selector (from the status tool)."),
  storageId: z.string().describe("The storage ID of the file to delete."),
});

const deleteOutputSchema = z.object({
  success: z.boolean().describe("Whether the file was successfully deleted."),
});

export const StorageDeleteTool: ConvexTool<
  typeof deleteInputSchema,
  typeof deleteOutputSchema
> = {
  name: "storage_delete",
  description:
    "Delete a file from Convex file storage by its storage ID. This is irreversible.",
  inputSchema: deleteInputSchema,
  outputSchema: deleteOutputSchema,
  handler: async (ctx, args) => {
    const { projectDir, deployment } = await ctx.decodeDeploymentSelector(
      args.deploymentSelector,
    );
    process.chdir(projectDir);
    const metadata = await getMcpDeploymentSelection(ctx, deployment);
    const credentials = await loadSelectedDeploymentCredentials(ctx, metadata);
    const logger = new DefaultLogger({ verbose: false });
    const client = new ConvexHttpClient(credentials.url, { logger });
    client.setAdminAuth(credentials.adminKey);

    try {
      await client.mutation(
        "_system/cli/storageDelete" as any,
        { storageId: args.storageId } as any,
      );
      return { success: true };
    } catch (err) {
      return await ctx.crash({
        exitCode: 1,
        errorType: "invalid filesystem or env vars",
        printedMessage: `Failed to delete file "${args.storageId}":\n${(err as Error).toString().trim()}`,
      });
    }
  },
};
