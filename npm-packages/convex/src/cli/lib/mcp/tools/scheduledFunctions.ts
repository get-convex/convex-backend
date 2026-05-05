import { z } from "zod";
import { ConvexTool } from "./index.js";
import { loadSelectedDeploymentCredentials } from "../../api.js";
import { runSystemQuery } from "../../run.js";
import { ConvexHttpClient } from "../../../../browser/index.js";
import { DefaultLogger } from "../../../../browser/logging.js";
import { getMcpDeploymentSelection } from "../requestContext.js";

// --- Crons List ---

const cronsInputSchema = z.object({
  deploymentSelector: z
    .string()
    .describe("Deployment selector (from the status tool)."),
});

const cronsOutputSchema = z.object({
  crons: z.array(
    z.object({
      name: z.string(),
      schedule: z.string(),
      functionName: z.string(),
    }),
  ),
});

export const CronsListTool: ConvexTool<
  typeof cronsInputSchema,
  typeof cronsOutputSchema
> = {
  name: "crons_list",
  description:
    "List all registered cron jobs in the Convex deployment, including their schedules and target functions.",
  inputSchema: cronsInputSchema,
  outputSchema: cronsOutputSchema,
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

    try {
      const result = (await runSystemQuery(ctx, {
        deploymentUrl: credentials.url,
        adminKey: credentials.adminKey,
        functionName: "_system/cli/tableData",
        componentPath: undefined,
        args: {
          table: "_cron_jobs",
          order: "asc",
          paginationOpts: {
            numItems: 100,
            cursor: null,
          },
        },
      })) as any;

      const crons = (result?.page ?? []).map((doc: any) => ({
        name: doc.name ?? doc._id,
        schedule: JSON.stringify(doc.schedule ?? doc.cronSpec),
        functionName: doc.functionName ?? doc.udfPath ?? "unknown",
      }));

      return { crons };
    } catch {
      return { crons: [] };
    }
  },
};

// --- Scheduled Functions List ---

const scheduledInputSchema = z.object({
  deploymentSelector: z
    .string()
    .describe("Deployment selector (from the status tool)."),
  status: z
    .enum(["pending", "inProgress", "completed", "failed", "canceled"])
    .optional()
    .describe("Filter by status. Defaults to showing all."),
  limit: z
    .number()
    .max(100)
    .optional()
    .describe("Maximum number of scheduled functions to return. Defaults to 50."),
});

const scheduledOutputSchema = z.object({
  scheduledFunctions: z.array(
    z.object({
      id: z.string(),
      functionName: z.string(),
      scheduledTime: z.string(),
      status: z.string(),
      args: z.any().optional(),
    }),
  ),
});

export const ScheduledListTool: ConvexTool<
  typeof scheduledInputSchema,
  typeof scheduledOutputSchema
> = {
  name: "scheduled_list",
  description:
    "List scheduled function calls in the Convex deployment. Can filter by status (pending, inProgress, completed, failed, canceled).",
  inputSchema: scheduledInputSchema,
  outputSchema: scheduledOutputSchema,
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

    try {
      const result = (await runSystemQuery(ctx, {
        deploymentUrl: credentials.url,
        adminKey: credentials.adminKey,
        functionName: "_system/cli/tableData",
        componentPath: undefined,
        args: {
          table: "_scheduled_functions",
          order: "desc",
          paginationOpts: {
            numItems: args.limit ?? 50,
            cursor: null,
          },
        },
      })) as any;

      let functions = (result?.page ?? []).map((doc: any) => ({
        id: doc._id,
        functionName: doc.udfPath ?? doc.name ?? "unknown",
        scheduledTime: doc.scheduledTime
          ? new Date(doc.scheduledTime).toISOString()
          : "unknown",
        status: doc.state?.kind ?? doc.status ?? "unknown",
        args: doc.args,
      }));

      if (args.status) {
        functions = functions.filter(
          (f: any) => f.status === args.status,
        );
      }

      return { scheduledFunctions: functions };
    } catch {
      return { scheduledFunctions: [] };
    }
  },
};

// --- Scheduled Function Cancel ---

const cancelInputSchema = z.object({
  deploymentSelector: z
    .string()
    .describe("Deployment selector (from the status tool)."),
  scheduledFunctionId: z
    .string()
    .describe("The _id of the scheduled function to cancel."),
});

const cancelOutputSchema = z.object({
  success: z.boolean().describe("Whether the cancellation was successful."),
});

export const ScheduledCancelTool: ConvexTool<
  typeof cancelInputSchema,
  typeof cancelOutputSchema
> = {
  name: "scheduled_cancel",
  description:
    "Cancel a pending scheduled function call by its _id. Only pending functions can be canceled.",
  inputSchema: cancelInputSchema,
  outputSchema: cancelOutputSchema,
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
        "_system/cli/cancelScheduledFunction" as any,
        { id: args.scheduledFunctionId } as any,
      );
      return { success: true };
    } catch (err) {
      return await ctx.crash({
        exitCode: 1,
        errorType: "invalid filesystem or env vars",
        printedMessage: `Failed to cancel scheduled function "${args.scheduledFunctionId}":\n${(err as Error).toString().trim()}`,
      });
    }
  },
};
