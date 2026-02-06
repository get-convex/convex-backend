import { z } from "zod";
import { ConvexTool } from "./index.js";
import {
  loadSelectedDeploymentCredentials,
  fetchTeamAndProject,
} from "../../api.js";
import { getDeploymentSelection } from "../../deploymentSelection.js";
import { bigBrainFetch, provisionHost } from "../../utils/utils.js";
import { deploymentDashboardUrlPage } from "../../../lib/dashboard.js";

const ROOT_COMPONENT_PATH = "-root-component-";
// Query ID for the insights dataset (shared with dashboard/src/api/insights.ts).
const INSIGHTS_QUERY_ID = "9ab3b74e-a725-480b-88a6-43e6bd70bd82";
const MAX_RECENT_EVENTS = 5;

const inputSchema = z.object({
  deploymentSelector: z
    .string()
    .describe(
      "Deployment selector (from the status tool) to fetch insights for.",
    ),
});

const occRecentEventSchema = z.object({
  timestamp: z.string(),
  id: z.string(),
  request_id: z.string(),
  occ_document_id: z.string().optional(),
  occ_write_source: z.string().optional(),
  occ_retry_count: z.number(),
});

const resourceRecentEventSchema = z.object({
  timestamp: z.string(),
  id: z.string(),
  request_id: z.string(),
  calls: z.array(
    z.object({
      table_name: z.string(),
      bytes_read: z.number(),
      documents_read: z.number(),
    }),
  ),
  success: z.boolean(),
});

const insightSchema = z.discriminatedUnion("kind", [
  z.object({
    kind: z.literal("occRetried"),
    severity: z.literal("warning"),
    functionId: z.string(),
    componentPath: z.string().nullable(),
    occCalls: z.number(),
    occTableName: z.string().optional(),
    recentEvents: z.array(occRecentEventSchema),
  }),
  z.object({
    kind: z.literal("occFailedPermanently"),
    severity: z.literal("error"),
    functionId: z.string(),
    componentPath: z.string().nullable(),
    occCalls: z.number(),
    occTableName: z.string().optional(),
    recentEvents: z.array(occRecentEventSchema),
  }),
  z.object({
    kind: z.literal("bytesReadLimit"),
    severity: z.literal("error"),
    functionId: z.string(),
    componentPath: z.string().nullable(),
    count: z.number(),
    recentEvents: z.array(resourceRecentEventSchema),
  }),
  z.object({
    kind: z.literal("bytesReadThreshold"),
    severity: z.literal("warning"),
    functionId: z.string(),
    componentPath: z.string().nullable(),
    count: z.number(),
    recentEvents: z.array(resourceRecentEventSchema),
  }),
  z.object({
    kind: z.literal("documentsReadLimit"),
    severity: z.literal("error"),
    functionId: z.string(),
    componentPath: z.string().nullable(),
    count: z.number(),
    recentEvents: z.array(resourceRecentEventSchema),
  }),
  z.object({
    kind: z.literal("documentsReadThreshold"),
    severity: z.literal("warning"),
    functionId: z.string(),
    componentPath: z.string().nullable(),
    count: z.number(),
    recentEvents: z.array(resourceRecentEventSchema),
  }),
]);

const outputSchema = z.object({
  insights: z.array(insightSchema),
  summary: z.string(),
  dashboardUrl: z.string(),
});

// Single source of truth: sorted from most to least severe.
const insightKinds: { kind: string; severity: "error" | "warning" }[] = [
  { kind: "documentsReadLimit", severity: "error" },
  { kind: "bytesReadLimit", severity: "error" },
  { kind: "occFailedPermanently", severity: "error" },
  { kind: "documentsReadThreshold", severity: "warning" },
  { kind: "bytesReadThreshold", severity: "warning" },
  { kind: "occRetried", severity: "warning" },
];

const insightKindMap = new Map(
  insightKinds.map((ik, i) => [ik.kind, { severity: ik.severity, order: i }]),
);

function orderForKind(kind: string): number {
  return insightKindMap.get(kind)?.order ?? insightKinds.length;
}

const description = `
Fetch health insights for a Convex deployment over the last 72 hours.

Returns OCC (Optimistic Concurrency Control) conflicts and resource limit issues
that may indicate performance problems or failing functions.

**OCC insights** (occRetried, occFailedPermanently):
  Mutations that conflict on the same document. To fix: restructure mutations to
  touch fewer shared documents, split hot documents, or reduce transaction scope.

**Resource limit insights** (bytesReadLimit, documentsReadLimit, bytesReadThreshold, documentsReadThreshold):
  Functions reading too much data. To fix: add indexes to avoid full table scans,
  use pagination, or filter data more precisely in queries.

Severity levels:
  - "error": Function executions are failing (permanent OCC failures or hard limits hit)
  - "warning": Function executions succeed but are at risk (retried OCCs or approaching limits)

Use the logs tool with status "failure" to see individual error messages and stack traces.

Only available for cloud deployments with user-level authentication.
`.trim();

export const InsightsTool: ConvexTool<typeof inputSchema, typeof outputSchema> =
  {
    name: "insights",
    description,
    inputSchema,
    outputSchema,
    handler: async (ctx, args) => {
      const { projectDir, deployment } = ctx.decodeDeploymentSelectorUnchecked(
        args.deploymentSelector,
      );
      process.chdir(projectDir);
      const deploymentSelection = await getDeploymentSelection(
        ctx,
        ctx.options,
      );
      const credentials = await loadSelectedDeploymentCredentials(
        ctx,
        deploymentSelection,
        deployment,
      );

      const deploymentName =
        credentials.deploymentFields?.deploymentName ?? null;
      if (deploymentName === null) {
        return await ctx.crash({
          exitCode: 1,
          errorType: "fatal",
          printedMessage:
            "Insights are only available for cloud deployments. Local deployments do not have insights data.",
        });
      }

      const auth = ctx.bigBrainAuth();
      if (
        auth === null ||
        auth.kind === "deploymentKey" ||
        auth.kind === "projectKey"
      ) {
        return await ctx.crash({
          exitCode: 1,
          errorType: "fatal",
          printedMessage:
            "Insights require user-level authentication. Deploy keys and project keys cannot access team usage data.",
        });
      }

      const { teamId } = await fetchTeamAndProject(ctx, deploymentName);

      const now = new Date();
      const hoursAgo72 = new Date(now.getTime() - 72 * 60 * 60 * 1000);
      const fromDate = hoursAgo72.toISOString().split("T")[0];
      const toDate = now.toISOString().split("T")[0];

      const queryParams = new URLSearchParams({
        queryId: INSIGHTS_QUERY_ID,
        deploymentName,
        from: fromDate,
        to: toDate,
      });
      const fetch = await bigBrainFetch(ctx);
      const res = await fetch(
        `dashboard/teams/${teamId}/usage/query?${queryParams.toString()}`,
        {
          method: "GET",
          headers: { Origin: provisionHost },
        },
      );
      const rawData = (await res.json()) as string[][];

      type Insight = z.infer<typeof insightSchema>;
      const insights: Insight[] = rawData.flatMap((row): Insight[] => {
        const kind = row[0];
        const functionId = row[1];
        const componentPath = row[2] === ROOT_COMPONENT_PATH ? null : row[2];
        const details = JSON.parse(row[3]);
        const recentEvents = (details.recentEvents as any[]).slice(
          0,
          MAX_RECENT_EVENTS,
        );
        const common = { functionId, componentPath };

        switch (kind) {
          case "occRetried":
            return [
              {
                kind,
                severity: "warning" as const,
                ...common,
                occCalls: details.occCalls,
                occTableName: details.occTableName,
                recentEvents,
              },
            ];
          case "occFailedPermanently":
            return [
              {
                kind,
                severity: "error" as const,
                ...common,
                occCalls: details.occCalls,
                occTableName: details.occTableName,
                recentEvents,
              },
            ];
          case "bytesReadLimit":
            return [
              {
                kind,
                severity: "error" as const,
                ...common,
                count: details.count,
                recentEvents,
              },
            ];
          case "bytesReadThreshold":
            return [
              {
                kind,
                severity: "warning" as const,
                ...common,
                count: details.count,
                recentEvents,
              },
            ];
          case "documentsReadLimit":
            return [
              {
                kind,
                severity: "error" as const,
                ...common,
                count: details.count,
                recentEvents,
              },
            ];
          case "documentsReadThreshold":
            return [
              {
                kind,
                severity: "warning" as const,
                ...common,
                count: details.count,
                recentEvents,
              },
            ];
          default:
            // Unknown insight kind — skip silently
            return [];
        }
      });

      insights.sort((a, b) => orderForKind(a.kind) - orderForKind(b.kind));

      const errorCount = insights.filter((i) => i.severity === "error").length;
      const warningCount = insights.filter(
        (i) => i.severity === "warning",
      ).length;

      let summary: string;
      if (insights.length === 0) {
        summary =
          "No issues found. The deployment is healthy over the last 72 hours.";
      } else {
        const parts: string[] = [];
        if (errorCount > 0)
          parts.push(`${errorCount} error${errorCount > 1 ? "s" : ""}`);
        if (warningCount > 0)
          parts.push(`${warningCount} warning${warningCount > 1 ? "s" : ""}`);
        summary = `Found ${parts.join(" and ")} in the last 72 hours.`;
      }

      const dashboardUrl = deploymentDashboardUrlPage(
        deploymentName,
        "/insights",
      );

      return { insights, summary, dashboardUrl };
    },
  };
