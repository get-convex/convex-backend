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
  teamSummaryByRegion: DatabricksQueryId;
  teamDeploymentCountByType: DatabricksQueryId;
} = {
  teamFunctionBreakdown: "8e6592dd-12a0-4ddf-bc79-7498e07352d4",
  teamSummaryByRegion: "36fc7cf3-a675-49f2-b1ce-23be09a712a2",
  teamDeploymentCountByType: "34801c2e-06a8-4cc5-8ecc-dd412b908763",
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
  teamDeploymentCountByProject: DatabricksQueryId;
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
  teamDeploymentCountByProject: "0b6c9ab3-c17c-4ad5-bfca-8f0300e494f6",
};

const DATABRICKS_BY_TABLE_QUERY_IDS: {
  teamDatabaseStorageByTable: DatabricksQueryId;
  teamDocumentCountByTable: DatabricksQueryId;
} = {
  teamDatabaseStorageByTable: "9cb8c431-6fa7-40cc-8f00-8aad358b4043",
  teamDocumentCountByTable: "1e265821-c0b9-4c66-9a1f-a5eef70e4d6f",
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
  const { data, error } = useUsageQuery({
    queryId: DATABRICKS_QUERY_IDS.teamSummaryByRegion,
    teamId,
    projectId,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  if (!data) {
    return { data: undefined, error: undefined };
  }

  // Report to sentry if this query returns the incorrect number of rows
  if (data.length < 1) {
    captureMessage(
      `Unexpected number of rows in usage summary query: ${data.length}`,
      "error",
    );
  }

  return {
    data: data?.map(
      ([
        _teamId,
        region,
        databaseStorage,
        databaseBandwidth,
        functionCalls,
        actionCompute,
        fileStorage,
        fileBandwidth,
        vectorStorage,
        vectorBandwidth,
      ]) => ({
        region,
        databaseStorage: Number(databaseStorage),
        databaseBandwidth: Number(databaseBandwidth),
        fileStorage: Number(fileStorage),
        fileBandwidth: Number(fileBandwidth),
        functionCalls: Number(functionCalls),
        actionCompute: Number(actionCompute) / 60 / 60, // Converts from GB-S to GB-H
        vectorStorage: Number(vectorStorage),
        vectorBandwidth: Number(vectorBandwidth),
      }),
    ),
    error: undefined,
  };
}

export type UsageSummary = {
  region: string;
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
): { data: AggregatedFunctionMetrics[] | undefined; error: any } {
  const { data, error } = useUsageQuery({
    queryId: DATABRICKS_QUERY_IDS.teamFunctionBreakdown,
    teamId,
    projectId,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  return {
    data: data?.map(
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
    ),
    error: undefined,
  };
}

export interface DailyPerTagMetrics {
  ds: string;
  metrics: { tag: string; value: number }[];
}

// By-project query hooks
export interface DailyMetricByProject extends DailyMetric {
  projectId: number | "_rest";
}

export interface DailyPerTagMetricsByProject extends DailyPerTagMetrics {
  projectId: number | "_rest";
}

export type DailyMetricByTable = {
  ds: string;
  projectId: number | "_rest";
  tableName: string;
  value: number;
};

function parseProjectId(projectId: string | number): number | "_rest" {
  if (projectId === "_rest") {
    return "_rest";
  }
  return Number(projectId);
}

export function useUsageTeamDocumentsPerDayByProject(
  teamId: number,
  period: DateRange | null,
  componentPrefix: string | null,
): { data: DailyMetricByProject[] | undefined; error: any } {
  const { data, error } = useUsageQuery({
    queryId: DATABRICKS_BY_PROJECT_QUERY_IDS.teamDocumentCountByProject,
    teamId,
    projectId: null,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  return {
    data: data?.map(([_teamId, projectId, ds, count]) => ({
      ds,
      projectId: parseProjectId(projectId),
      value: Number(count),
    })),
    error: undefined,
  };
}

export function useUsageTeamDatabaseBandwidthPerDayByProject(
  teamId: number,
  period: DateRange | null,
  componentPrefix: string | null,
): { data: DailyPerTagMetricsByProject[] | undefined; error: any } {
  const { data, error } = useUsageQuery({
    queryId: DATABRICKS_BY_PROJECT_QUERY_IDS.teamDatabaseBandwidthByProject,
    teamId,
    projectId: null,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  return {
    data: data?.map(([_teamId, projectId, ds, ingressSize, egressSize]) => ({
      ds,
      projectId: parseProjectId(projectId),
      metrics: [
        { tag: "egress", value: Number(egressSize) },
        {
          tag: "ingress",
          value: Number(ingressSize),
        },
      ],
    })),
    error: undefined,
  };
}

export function useUsageTeamVectorStoragePerDayByProject(
  teamId: number,
  period: DateRange | null,
  componentPrefix: string | null,
): { data: DailyMetricByProject[] | undefined; error: any } {
  const { data, error } = useUsageQuery({
    queryId: DATABRICKS_BY_PROJECT_QUERY_IDS.teamVectorStorageByProject,
    teamId,
    projectId: null,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  return {
    data: data?.map(([_teamId, projectId, ds, vectorStorage]) => ({
      ds,
      projectId: parseProjectId(projectId),
      value: Number(vectorStorage),
    })),
    error: undefined,
  };
}

export function useUsageTeamVectorBandwidthPerDayByProject(
  teamId: number,
  period: DateRange | null,
  componentPrefix: string | null,
): { data: DailyPerTagMetricsByProject[] | undefined; error: any } {
  const { data, error } = useUsageQuery({
    queryId: DATABRICKS_BY_PROJECT_QUERY_IDS.teamVectorBandwidthByProject,
    teamId,
    projectId: null,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  return {
    data: data?.map(([_teamId, projectId, ds, ingressSize, egressSize]) => ({
      ds,
      projectId: parseProjectId(projectId),
      metrics: [
        { tag: "egress", value: Number(egressSize) },
        {
          tag: "ingress",
          value: Number(ingressSize),
        },
      ],
    })),
    error: undefined,
  };
}

export function useUsageTeamDatabaseStoragePerDayByProject(
  teamId: number,
  period: DateRange | null,
  componentPrefix: string | null,
): { data: DailyPerTagMetricsByProject[] | undefined; error: any } {
  const { data, error } = useUsageQuery({
    queryId: DATABRICKS_BY_PROJECT_QUERY_IDS.teamDatabaseStorageByProject,
    teamId,
    projectId: null,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  return {
    data: data?.map(
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
    ),
    error: undefined,
  };
}

export function useUsageTeamActionComputeDailyByProject(
  teamId: number,
  period: DateRange | null,
  componentPrefix: string | null,
): { data: DailyMetricByProject[] | undefined; error: any } {
  const { data, error } = useUsageQuery({
    queryId: DATABRICKS_BY_PROJECT_QUERY_IDS.teamActionComputeByProject,
    teamId,
    projectId: null,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  return {
    data: data?.map(([_teamId, projectId, ds, valueGbS]) => {
      const valueGbHour = Number(valueGbS) / 60 / 60;
      return {
        ds,
        projectId: parseProjectId(projectId),
        value: valueGbHour,
      };
    }),
    error: undefined,
  };
}

export function useUsageTeamDailyCallsByTagByProject(
  teamId: number,
  period: DateRange | null,
  componentPrefix: string | null,
): { data: DailyPerTagMetricsByProject[] | undefined; error: any } {
  const { data: functionData, error: functionError } = useUsageQuery({
    queryId: DATABRICKS_BY_PROJECT_QUERY_IDS.teamFunctionCallsByProject,
    teamId,
    projectId: null,
    period,
    componentPrefix,
  });

  const { data: storageData, error: storageError } = useUsageQuery({
    queryId: DATABRICKS_BY_PROJECT_QUERY_IDS.teamStorageCallsByProject,
    teamId,
    projectId: null,
    period,
    componentPrefix,
  });

  if (functionError || storageError) {
    return { data: undefined, error: functionError || storageError };
  }

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

  return { data: metrics, error: undefined };
}

export function useUsageTeamStoragePerDayByProject(
  teamId: number,
  period: DateRange | null,
  componentPrefix: string | null,
): { data: DailyPerTagMetricsByProject[] | undefined; error: any } {
  const { data, error } = useUsageQuery({
    queryId: DATABRICKS_BY_PROJECT_QUERY_IDS.teamFileStorageByProject,
    teamId,
    projectId: null,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  return {
    data: data?.map(
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
    ),
    error: undefined,
  };
}

export function useUsageTeamStorageThroughputDailyByProject(
  teamId: number,
  period: DateRange | null,
  componentPrefix: string | null,
): { data: DailyPerTagMetricsByProject[] | undefined; error: any } {
  const { data, error } = useUsageQuery({
    queryId: DATABRICKS_BY_PROJECT_QUERY_IDS.teamFileBandwidthByProject,
    teamId,
    projectId: null,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  return {
    data: data?.map(
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
    ),
    error: undefined,
  };
}

export function useUsageTeamDeploymentCountPerDayByProject(
  teamId: number,
  period: DateRange | null,
  componentPrefix: string | null,
): { data: DailyMetricByProject[] | undefined; error: any } {
  const { data, error } = useUsageQuery({
    queryId: DATABRICKS_BY_PROJECT_QUERY_IDS.teamDeploymentCountByProject,
    teamId,
    projectId: null,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  return {
    data: data?.map(([_teamId, projectId, ds, count]) => ({
      ds,
      projectId: parseProjectId(projectId),
      value: Number(count),
    })),
    error: undefined,
  };
}

export function useUsageTeamDeploymentCountByType(
  teamId: number,
  period: DateRange | null,
  projectId: number | null,
  componentPrefix: string | null,
): { data: DailyPerTagMetrics[] | undefined; error: any } {
  const { data, error } = useUsageQuery({
    queryId: DATABRICKS_QUERY_IDS.teamDeploymentCountByType,
    teamId,
    projectId,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  if (data === undefined) {
    return { data: undefined, error: undefined };
  }

  // Group by date since each row is [teamId, deploymentType, ds, count]
  const groupedByDate = new Map<string, Map<string, number>>();

  data.forEach(([_teamId, deploymentType, ds, count]) => {
    if (!groupedByDate.has(ds)) {
      groupedByDate.set(ds, new Map());
    }
    const tag = deploymentType || "deleted";
    groupedByDate.get(ds)!.set(tag, Number(count));
  });

  return {
    data: Array.from(groupedByDate.entries()).map(([ds, metricsMap]) => ({
      ds,
      metrics: Array.from(metricsMap.entries()).map(([tag, value]) => ({
        tag,
        value,
      })),
    })),
    error: undefined,
  };
}

// By-table query hooks
export function useUsageTeamDatabaseStoragePerDayByTable(
  teamId: number,
  period: DateRange | null,
  projectId: number | null,
  componentPrefix: string | null,
): { data: DailyMetricByTable[] | undefined; error: any } {
  const { data, error } = useUsageQuery({
    queryId: DATABRICKS_BY_TABLE_QUERY_IDS.teamDatabaseStorageByTable,
    teamId,
    projectId,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  return {
    data: data?.map(
      ([
        _teamId,
        rowProjectId,
        tableName,
        ds,
        documentStorage,
        indexStorage,
      ]) => ({
        ds,
        projectId: parseProjectId(rowProjectId),
        tableName,
        value: Number(documentStorage) + Number(indexStorage),
      }),
    ),
    error: undefined,
  };
}

export function useUsageTeamDocumentCountPerDayByTable(
  teamId: number,
  period: DateRange | null,
  projectId: number | null,
  componentPrefix: string | null,
): { data: DailyMetricByTable[] | undefined; error: any } {
  const { data, error } = useUsageQuery({
    queryId: DATABRICKS_BY_TABLE_QUERY_IDS.teamDocumentCountByTable,
    teamId,
    projectId,
    period,
    componentPrefix,
  });

  if (error) {
    return { data: undefined, error };
  }

  return {
    data: data?.map(
      ([_teamId, rowProjectId, tableName, ds, documentCount]) => ({
        ds,
        projectId: parseProjectId(rowProjectId),
        tableName,
        value: Number(documentCount),
      }),
    ),
    error: undefined,
  };
}
