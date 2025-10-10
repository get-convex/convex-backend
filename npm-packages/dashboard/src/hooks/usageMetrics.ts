import { captureMessage } from "@sentry/nextjs";
import { useBBQuery } from "api/api";
import {
  DatabricksQueryId,
  DateRange,
  USAGE_REFRESH_INTERVAL_MS,
  useUsageQuery,
} from "api/usage";

const DATABRICKS_QUERY_IDS: {
  teamFunctionBreakdown: DatabricksQueryId;
  teamSummary: DatabricksQueryId;
} = {
  teamFunctionBreakdown: "8e6592dd-12a0-4ddf-bc79-7498e07352d4",
  teamSummary: "15fbb132-6641-4f17-9156-b05e9ee966d9",
};

const DATABRICKS_BY_PROJECT_QUERY_IDS: {
  teamActionComputeByProject: DatabricksQueryId;
  teamDatabaseBandwidthByProject: DatabricksQueryId;
  teamDocumentCountByProject: DatabricksQueryId;
  teamDatabaseStorageByProject: DatabricksQueryId;
  teamFileBandwidthByProject: DatabricksQueryId;
  teamFileStorageByProject: DatabricksQueryId;
  teamFunctionCallsByProject: DatabricksQueryId;
  teamStorageCallsByProject: DatabricksQueryId;
  teamVectorBandwidthByProject: DatabricksQueryId;
  teamVectorStorageByProject: DatabricksQueryId;
} = {
  teamActionComputeByProject: "56e7167c-ae79-417a-8876-100a6e5db902",
  teamDatabaseBandwidthByProject: "27330248-82cd-42dd-bc23-a7edc667e1ba",
  teamDocumentCountByProject: "2a5120a1-b334-4d99-b378-1028487c2202",
  teamDatabaseStorageByProject: "25ee9158-a11c-45f9-8fd7-c9262c81e99f",
  teamFileBandwidthByProject: "bc8ac376-b0fd-4745-a1b3-ed7bf73f7825",
  teamFileStorageByProject: "e4cb8bdc-f54e-497d-947e-b373d51fe86a",
  teamFunctionCallsByProject: "2cc73be2-3eb0-4069-ad44-749c619b0aa4",
  teamStorageCallsByProject: "11d0124f-32c9-446f-b65c-cde3971d2017",
  teamVectorBandwidthByProject: "256e0060-8d24-4d30-968c-d6f531168328",
  teamVectorStorageByProject: "6f3c9a52-f3c2-46c2-89b2-6cb95fc7cdf5",
};

export function useTokenUsage(teamSlug: string, period: DateRange | null) {
  return useBBQuery({
    path: "/teams/{team_slug}/usage/get_token_info",
    pathParams: { team_slug: teamSlug },
    queryParams: period ? { from: period.from, to: period.to } : undefined,
    swrOptions: {
      keepPreviousData: false,
      refreshInterval: USAGE_REFRESH_INTERVAL_MS,
    },
  });
}

export function useUsageTeamSummary(
  teamId: number,
  period: DateRange | null,
  projectId: number | null,
  componentPrefix: string | null,
) {
  const { data } = useUsageQuery({
    queryId: DATABRICKS_QUERY_IDS.teamSummary,
    teamId,
    projectId,
    period,
    componentPrefix,
  });

  if (!data) {
    return undefined;
  }

  // Report to sentry if this query returns the incorrect number of rows
  if (data.length !== 1) {
    captureMessage(
      `Unexpected number of rows in usage summary query: ${data.length}`,
      "error",
    );
  }

  return data?.map(
    ([
      _teamId,
      databaseStorage,
      databaseBandwidth,
      functionCalls,
      actionCompute,
      fileStorage,
      fileBandwidth,
      vectorStorage,
      vectorBandwidth,
    ]) => ({
      databaseStorage: Number(databaseStorage),
      databaseBandwidth: Number(databaseBandwidth),
      fileStorage: Number(fileStorage),
      fileBandwidth: Number(fileBandwidth),
      functionCalls: Number(functionCalls),
      actionCompute: Number(actionCompute) / 60 / 60, // Converts from GB-S to GB-H
      vectorStorage: Number(vectorStorage),
      vectorBandwidth: Number(vectorBandwidth),
    }),
  )[0];
}

export type UsageSummary = {
  databaseStorage: number;
  databaseBandwidth: number;
  fileStorage: number;
  fileBandwidth: number;
  functionCalls: number;
  actionCompute: number;
  vectorStorage: number;
  vectorBandwidth: number;
};

export type DailyMetric = {
  ds: string;
  value: number;
};

export interface AggregatedFunctionMetrics {
  function: string;
  projectId: number;
  callCount: number;
  databaseIngressSize: number;
  databaseEgressSize: number;
  vectorIngressSize: number;
  vectorEgressSize: number;
  actionComputeTime: number;
  deploymentId?: number;
  deploymentName?: string;
  componentPath: string;
}

export function useUsageTeamMetricsByFunction(
  teamId: number,
  period: DateRange | null,
  projectId: number | null,
  componentPrefix: string | null,
): AggregatedFunctionMetrics[] | undefined {
  const { data } = useUsageQuery({
    queryId: DATABRICKS_QUERY_IDS.teamFunctionBreakdown,
    teamId,
    projectId,
    period,
    componentPrefix,
  });

  return data?.map(
    ([
      _teamId,
      functionName,
      projectIdField,
      callCount,
      databaseIngressSize,
      databaseEgressSize,
      vectorIngressSize,
      vectorEgressSize,
      actionComputeTime,
      deploymentName,
      componentPath,
    ]) => ({
      function: functionName,
      projectId: Number(projectIdField),
      callCount: Number(callCount),
      databaseIngressSize: Number(databaseIngressSize),
      databaseEgressSize: Number(databaseEgressSize),
      vectorIngressSize: Number(vectorIngressSize),
      vectorEgressSize: Number(vectorEgressSize),
      actionComputeTime: Number(actionComputeTime) / 60 / 60, // Converts from GB-S to GB-H
      deploymentName,
      componentPath,
    }),
  );
}

export interface DailyPerTagMetrics {
  ds: string;
  metrics: { tag: string; value: number }[];
}

// By-project query hooks
export interface DailyMetricByProject extends DailyMetric {
  projectId: number | string; // Can be a number or "_rest"
}

export interface DailyPerTagMetricsByProject extends DailyPerTagMetrics {
  projectId: number | string; // Can be a number or "_rest"
}

function parseProjectId(projectId: string | number): number | string {
  if (projectId === "_rest") {
    return "_rest";
  }
  return Number(projectId);
}

export function useUsageTeamDocumentsPerDayByProject(
  teamId: number,
  period: DateRange | null,
  componentPrefix: string | null,
): DailyMetricByProject[] | undefined {
  const { data } = useUsageQuery({
    queryId: DATABRICKS_BY_PROJECT_QUERY_IDS.teamDocumentCountByProject,
    teamId,
    projectId: null,
    period,
    componentPrefix,
  });

  return data?.map(([_teamId, projectId, ds, count]) => ({
    ds,
    projectId: parseProjectId(projectId),
    value: Number(count),
  }));
}

export function useUsageTeamDatabaseBandwidthPerDayByProject(
  teamId: number,
  period: DateRange | null,
  componentPrefix: string | null,
): DailyPerTagMetricsByProject[] | undefined {
  const { data } = useUsageQuery({
    queryId: DATABRICKS_BY_PROJECT_QUERY_IDS.teamDatabaseBandwidthByProject,
    teamId,
    projectId: null,
    period,
    componentPrefix,
  });
  return data?.map(([_teamId, projectId, ds, ingressSize, egressSize]) => ({
    ds,
    projectId: parseProjectId(projectId),
    metrics: [
      { tag: "egress", value: Number(egressSize) },
      {
        tag: "ingress",
        value: Number(ingressSize),
      },
    ],
  }));
}

export function useUsageTeamVectorStoragePerDayByProject(
  teamId: number,
  period: DateRange | null,
  componentPrefix: string | null,
): DailyMetricByProject[] | undefined {
  const { data } = useUsageQuery({
    queryId: DATABRICKS_BY_PROJECT_QUERY_IDS.teamVectorStorageByProject,
    teamId,
    projectId: null,
    period,
    componentPrefix,
  });
  return data?.map(([_teamId, projectId, ds, vectorStorage]) => ({
    ds,
    projectId: parseProjectId(projectId),
    value: Number(vectorStorage),
  }));
}

export function useUsageTeamVectorBandwidthPerDayByProject(
  teamId: number,
  period: DateRange | null,
  componentPrefix: string | null,
): DailyPerTagMetricsByProject[] | undefined {
  const { data } = useUsageQuery({
    queryId: DATABRICKS_BY_PROJECT_QUERY_IDS.teamVectorBandwidthByProject,
    teamId,
    projectId: null,
    period,
    componentPrefix,
  });
  return data?.map(([_teamId, projectId, ds, ingressSize, egressSize]) => ({
    ds,
    projectId: parseProjectId(projectId),
    metrics: [
      { tag: "egress", value: Number(egressSize) },
      {
        tag: "ingress",
        value: Number(ingressSize),
      },
    ],
  }));
}

export function useUsageTeamDatabaseStoragePerDayByProject(
  teamId: number,
  period: DateRange | null,
  componentPrefix: string | null,
): DailyPerTagMetricsByProject[] | undefined {
  const { data } = useUsageQuery({
    queryId: DATABRICKS_BY_PROJECT_QUERY_IDS.teamDatabaseStorageByProject,
    teamId,
    projectId: null,
    period,
    componentPrefix,
  });
  return data?.map(
    ([_teamId, projectId, ds, documentStorage, indexStorage]) => ({
      ds,
      projectId: parseProjectId(projectId),
      metrics: [
        { tag: "document", value: Number(documentStorage) },
        {
          tag: "index",
          value: Number(indexStorage),
        },
      ],
    }),
  );
}

export function useUsageTeamActionComputeDailyByProject(
  teamId: number,
  period: DateRange | null,
  componentPrefix: string | null,
): DailyMetricByProject[] | undefined {
  const { data } = useUsageQuery({
    queryId: DATABRICKS_BY_PROJECT_QUERY_IDS.teamActionComputeByProject,
    teamId,
    projectId: null,
    period,
    componentPrefix,
  });
  return data?.map(([_teamId, projectId, ds, valueGbS]) => {
    const valueGbHour = Number(valueGbS) / 60 / 60;
    return {
      ds,
      projectId: parseProjectId(projectId),
      value: valueGbHour,
    };
  });
}

export function useUsageTeamDailyCallsByTagByProject(
  teamId: number,
  period: DateRange | null,
  componentPrefix: string | null,
): DailyPerTagMetricsByProject[] | undefined {
  const { data: functionData } = useUsageQuery({
    queryId: DATABRICKS_BY_PROJECT_QUERY_IDS.teamFunctionCallsByProject,
    teamId,
    projectId: null,
    period,
    componentPrefix,
  });

  const { data: storageData } = useUsageQuery({
    queryId: DATABRICKS_BY_PROJECT_QUERY_IDS.teamStorageCallsByProject,
    teamId,
    projectId: null,
    period,
    componentPrefix,
  });

  // Start with functionData
  const metrics = functionData?.map(
    ([
      _teamId,
      projectId,
      ds,
      cachedQueries,
      uncachedQueries,
      mutations,
      actions,
      httpActions,
    ]) => ({
      ds,
      projectId: parseProjectId(projectId),
      metrics: [
        { tag: "uncached_query", value: Number(uncachedQueries) },
        {
          tag: "cached_query",
          value: Number(cachedQueries),
        },
        {
          tag: "mutation",
          value: Number(mutations),
        },
        {
          tag: "action",
          value: Number(actions),
        },
        {
          tag: "http_action",
          value: Number(httpActions),
        },
      ],
    }),
  );

  // Augment with storage data
  const storageDataByDsAndProject = (storageData || []).reduce(
    (acc, [_teamId, projectId, ds, storageCalls]) => {
      const key = `${ds}-${projectId}`;
      acc[key] = Number(storageCalls);
      return acc;
    },
    {} as Record<string, number>,
  );
  for (const metric of metrics || []) {
    const key = `${metric.ds}-${metric.projectId}`;
    const storageCalls = storageDataByDsAndProject[key];
    if (storageCalls) {
      metric.metrics.push({ tag: "storage_api", value: storageCalls });
      delete storageDataByDsAndProject[key];
    }
  }

  return metrics;
}

export function useUsageTeamStoragePerDayByProject(
  teamId: number,
  period: DateRange | null,
  componentPrefix: string | null,
): DailyPerTagMetricsByProject[] | undefined {
  const { data } = useUsageQuery({
    queryId: DATABRICKS_BY_PROJECT_QUERY_IDS.teamFileStorageByProject,
    teamId,
    projectId: null,
    period,
    componentPrefix,
  });
  return data?.map(
    ([
      _teamId,
      projectId,
      ds,
      _totalFileSize,
      userFileSize,
      cloudBackupSize,
    ]) => ({
      ds,
      projectId: parseProjectId(projectId),
      metrics: [
        { tag: "userFiles", value: Number(userFileSize) },
        {
          tag: "cloudBackup",
          value: Number(cloudBackupSize),
        },
      ],
    }),
  );
}

export function useUsageTeamStorageThroughputDailyByProject(
  teamId: number,
  period: DateRange | null,
  componentPrefix: string | null,
): DailyPerTagMetricsByProject[] | undefined {
  const { data } = useUsageQuery({
    queryId: DATABRICKS_BY_PROJECT_QUERY_IDS.teamFileBandwidthByProject,
    teamId,
    projectId: null,
    period,
    componentPrefix,
  });
  return data?.map(
    ([
      _teamId,
      projectId,
      ds,
      servingIngressSize,
      servingEgressSize,
      userFunctionIngressSize,
      userFunctionEgressSize,
      cloudBackupSize,
      cloudRestoreSize,
      snapshotExportSize,
      snapshotImportSize,
    ]) => ({
      ds,
      projectId: parseProjectId(projectId),
      metrics: [
        { tag: "servingEgress", value: Number(servingEgressSize) },
        {
          tag: "servingIngress",
          value: Number(servingIngressSize),
        },
        {
          tag: "userFunctionEgress",
          value: Number(userFunctionEgressSize),
        },
        {
          tag: "userFunctionIngress",
          value: Number(userFunctionIngressSize),
        },
        {
          tag: "cloudRestore",
          value: Number(cloudRestoreSize),
        },
        {
          tag: "cloudBackup",
          value: Number(cloudBackupSize),
        },
        {
          tag: "snapshotExport",
          value: Number(snapshotExportSize),
        },
        {
          tag: "snapshotImport",
          value: Number(snapshotImportSize),
        },
      ],
    }),
  );
}
