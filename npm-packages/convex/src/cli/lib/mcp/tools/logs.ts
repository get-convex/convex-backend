import { z } from "zod";
import { ConvexTool } from "./index.js";
import { loadSelectedDeploymentCredentials } from "../../api.js";
import { getDeploymentSelection } from "../../deploymentSelection.js";
import { deploymentFetch } from "../../utils/utils.js";

const inputSchema = z.object({
  deploymentSelector: z
    .string()
    .describe("Deployment selector (from the status tool) to read logs from."),
  cursor: z
    .number()
    .optional()
    .describe(
      "Optional cursor (in ms) to start reading from. Use 0 to read from the beginning.",
    ),
  limit: z
    .number()
    .int()
    .positive()
    .max(1000)
    .optional()
    .describe(
      "Maximum number of log entries to return. If omitted, returns all available in this chunk.",
    ),
});

const structuredLogLine = z.object({
  messages: z.array(z.string()),
  level: z.enum(["LOG", "DEBUG", "INFO", "WARN", "ERROR"]),
  timestamp: z.number(),
  isTruncated: z.boolean(),
});

const logEntry = z.object({
  identifier: z.string(),
  udfType: z.enum(["Query", "Mutation", "Action", "HttpAction"]),
  logLines: z.array(z.union([z.string(), structuredLogLine])).optional(),
  timestamp: z.number(),
  executionTime: z.number(),
  error: z.string().nullable(),
  kind: z.enum(["Completion", "Progress"]),
});

const outputSchema = z.object({
  entries: z.array(logEntry),
  newCursor: z.number(),
});

const description = `
Fetch a chunk of recent log entries from your Convex deployment.

Returns a batch of UDF execution log entries and a new cursor you can use to
request the next batch. This tool does not tail; it performs a single fetch.
`.trim();

export const LogsTool: ConvexTool<typeof inputSchema, typeof outputSchema> = {
  name: "logs",
  description,
  inputSchema,
  outputSchema,
  handler: async (ctx, args) => {
    const { projectDir, deployment } = await ctx.decodeDeploymentSelector(
      args.deploymentSelector,
    );
    process.chdir(projectDir);
    const deploymentSelection = await getDeploymentSelection(ctx, ctx.options);
    const credentials = await loadSelectedDeploymentCredentials(
      ctx,
      deploymentSelection,
      deployment,
    );

    const fetch = deploymentFetch(ctx, {
      deploymentUrl: credentials.url,
      adminKey: credentials.adminKey,
    });

    const cursor = args.cursor ?? 0;
    const response = await fetch(`/api/stream_function_logs?cursor=${cursor}`, {
      method: "GET",
    });
    if (!response.ok) {
      return await ctx.crash({
        exitCode: 1,
        errorType: "fatal",
        printedMessage: `HTTP error ${response.status}: ${await response.text()}`,
      });
    }
    const { entries, newCursor } = (await response.json()) as {
      entries: unknown[];
      newCursor: number;
    };

    // Optionally limit the number of entries returned from the end.
    const limitedEntries =
      typeof args.limit === "number" && entries.length > args.limit
        ? entries.slice(entries.length - args.limit)
        : entries;

    const parsed = outputSchema.parse({ entries: limitedEntries, newCursor });
    return parsed;
  },
};
