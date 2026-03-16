import { z } from "zod";
import { ConvexTool } from "./index.js";
import { loadSelectedDeploymentCredentials } from "../../api.js";
import { runSystemQuery } from "../../run.js";
import { ConvexHttpClient } from "../../../../browser/index.js";
import { DefaultLogger } from "../../../../browser/logging.js";
import { getMcpDeploymentSelection } from "../requestContext.js";

// --- Get Document by ID ---

const getInputSchema = z.object({
  deploymentSelector: z
    .string()
    .describe("Deployment selector (from the status tool)."),
  tableName: z.string().describe("The name of the table to read from."),
  documentId: z.string().describe("The _id of the document to retrieve."),
});

const getOutputSchema = z.object({
  document: z.any().describe("The full document, or null if not found."),
});

export const GetDocumentTool: ConvexTool<
  typeof getInputSchema,
  typeof getOutputSchema
> = {
  name: "get_document",
  description:
    "Get a single document by its _id from a table in the Convex deployment. Returns the full document or null if not found.",
  inputSchema: getInputSchema,
  outputSchema: getOutputSchema,
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
    const result = await runSystemQuery(ctx, {
      deploymentUrl: credentials.url,
      adminKey: credentials.adminKey,
      functionName: "_system/cli/queryTable",
      componentPath: undefined,
      args: {
        table: args.tableName,
        filters: [{ field: "_id", op: "eq", value: args.documentId }],
        limit: 1,
      },
    });
    const docs = (result as any)?.page ?? [];
    return { document: docs.length > 0 ? docs[0] : null };
  },
};

// --- Insert Document ---

const insertInputSchema = z.object({
  deploymentSelector: z
    .string()
    .describe("Deployment selector (from the status tool)."),
  tableName: z.string().describe("The name of the table to insert into."),
  document: z
    .string()
    .describe(
      "The document to insert, JSON-encoded as a string. Must match the table schema.",
    ),
});

const insertOutputSchema = z.object({
  documentId: z.string().describe("The _id of the newly inserted document."),
});

export const InsertDocumentTool: ConvexTool<
  typeof insertInputSchema,
  typeof insertOutputSchema
> = {
  name: "insert_document",
  description:
    "Insert a new document into a table in the Convex deployment. The document must match the table's schema. Use the 'tables' tool first to see available fields and types.",
  inputSchema: insertInputSchema,
  outputSchema: insertOutputSchema,
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

    let parsedDoc: Record<string, any>;
    try {
      parsedDoc = JSON.parse(args.document);
    } catch {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: "Invalid JSON in document argument.",
      });
    }

    try {
      const result = await client.mutation(
        "_system/cli/tableInsert" as any,
        { table: args.tableName, document: parsedDoc } as any,
      );
      return { documentId: String(result) };
    } catch (err) {
      return await ctx.crash({
        exitCode: 1,
        errorType: "invalid filesystem or env vars",
        printedMessage: `Failed to insert document into "${args.tableName}":\n${(err as Error).toString().trim()}`,
      });
    }
  },
};

// --- Patch Document ---

const patchInputSchema = z.object({
  deploymentSelector: z
    .string()
    .describe("Deployment selector (from the status tool)."),
  tableName: z.string().describe("The name of the table."),
  documentId: z.string().describe("The _id of the document to patch."),
  fields: z
    .string()
    .describe(
      "The fields to update, JSON-encoded as a string. Only specified fields will be changed.",
    ),
});

const patchOutputSchema = z.object({
  success: z.boolean().describe("Whether the patch was successful."),
});

export const PatchDocumentTool: ConvexTool<
  typeof patchInputSchema,
  typeof patchOutputSchema
> = {
  name: "patch_document",
  description:
    "Update specific fields on an existing document by _id. Only the specified fields will be changed; other fields remain untouched. Use the 'tables' tool first to see available fields.",
  inputSchema: patchInputSchema,
  outputSchema: patchOutputSchema,
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

    let parsedFields: Record<string, any>;
    try {
      parsedFields = JSON.parse(args.fields);
    } catch {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: "Invalid JSON in fields argument.",
      });
    }

    try {
      await client.mutation("_system/cli/tablePatch" as any, {
        table: args.tableName,
        id: args.documentId,
        fields: parsedFields,
      } as any);
      return { success: true };
    } catch (err) {
      return await ctx.crash({
        exitCode: 1,
        errorType: "invalid filesystem or env vars",
        printedMessage: `Failed to patch document "${args.documentId}" in "${args.tableName}":\n${(err as Error).toString().trim()}`,
      });
    }
  },
};

// --- Delete Document ---

const deleteInputSchema = z.object({
  deploymentSelector: z
    .string()
    .describe("Deployment selector (from the status tool)."),
  tableName: z.string().describe("The name of the table."),
  documentId: z.string().describe("The _id of the document to delete."),
});

const deleteOutputSchema = z.object({
  success: z.boolean().describe("Whether the deletion was successful."),
});

export const DeleteDocumentTool: ConvexTool<
  typeof deleteInputSchema,
  typeof deleteOutputSchema
> = {
  name: "delete_document",
  description:
    "Delete a document by its _id from a table in the Convex deployment. This is irreversible.",
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
      await client.mutation("_system/cli/tableDelete" as any, {
        table: args.tableName,
        id: args.documentId,
      } as any);
      return { success: true };
    } catch (err) {
      return await ctx.crash({
        exitCode: 1,
        errorType: "invalid filesystem or env vars",
        printedMessage: `Failed to delete document "${args.documentId}" from "${args.tableName}":\n${(err as Error).toString().trim()}`,
      });
    }
  },
};
