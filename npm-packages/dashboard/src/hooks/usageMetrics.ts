import { captureMessage } from "@sentry/nextjs";
import { DatabricksQueryId, DateRange, useUsageQuery } from "api/usage";

const DATABRICKS_QUERY_IDS: {
  teamActionCompute: DatabricksQueryId;
  teamDatabaseBandwidth: DatabricksQueryId;
  teamDocumentCount: DatabricksQueryId;
  teamDatabaseStorage: DatabricksQueryId;
  teamFileBandwidth: DatabricksQueryId;
  teamFileStorage: DatabricksQueryId;
  teamFunctionBreakdown: DatabricksQueryId;
  teamFunctionCalls: DatabricksQueryId;
  teamStorageCalls: DatabricksQueryId;
  teamSummary: DatabricksQueryId;
  teamVectorBandwidth: DatabricksQueryId;
  teamVectorStorage: DatabricksQueryId;
} = {
  teamActionCompute: "544ac7ed-a3bc-43b6-9ee1-a8ef6ae283a9",
  teamDatabaseBandwidth: "20db8d1c-d08c-41da-93c6-5cecb6b97118",
  teamDocumentCount: "da7e013a-3042-48a4-ad85-cc3f035a035e",
  teamDatabaseStorage: "051e19e8-d9bf-4a80-81d1-f10c92b94ee6",
  teamFileBandwidth: "c9d757fb-7372-4d6a-9a8a-66ee7436ed47",
  teamFileStorage: "d0b4f882-48f5-4ad7-99e7-0b18f16355eb",
  teamFunctionBreakdown: "8e6592dd-12a0-4ddf-bc79-7498e07352d4",
  teamFunctionCalls: "46aa42bb-1f90-4fb5-8466-10bc52fcb43f",
  teamStorageCalls: "fe187e75-8670-4c16-a5c4-2cf7b0c5406f",
  teamSummary: "15fbb132-6641-4f17-9156-b05e9ee966d9",
  teamVectorBandwidth: "e24b4660-5dc4-4e41-a895-a91a66dede80",
  teamVectorStorage: "6cf7ee95-c39e-419e-ac3e-cb0acfcc2a0b",
};

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

export function useUsageTeamDocumentsPerDay(
  teamId: number,
  projectId: number | null,
  period: DateRange | null,
  componentPrefix: string | null,
): DailyMetric[] | undefined {
  const { data } = useUsageQuery({
    queryId: DATABRICKS_QUERY_IDS.teamDocumentCount,
    teamId,
    projectId,
    period,
    componentPrefix,
  });

  return data?.map(([_teamId, ds, count]) => ({
    ds,
    value: Number(count),
  }));
}

export function useUsageTeamDatabaseBandwidthPerDay(
  teamId: number,
  projectId: number | null,
  period: DateRange | null,
  componentPrefix: string | null,
) {
  const { data } = useUsageQuery({
    queryId: DATABRICKS_QUERY_IDS.teamDatabaseBandwidth,
    teamId,
    projectId,
    period,
    componentPrefix,
  });
  return data?.map(([_teamId, ds, ingressSize, egressSize]) => ({
    ds,
    metrics: [
      { tag: "ingress", value: Number(ingressSize) },
      {
        tag: "egress",
        value: Number(egressSize),
      },
    ],
  }));
}

export function useUsageTeamVectorStoragePerDay(
  teamId: number,
  projectId: number | null,
  period: DateRange | null,
  componentPrefix: string | null,
): DailyMetric[] | undefined {
  const { data } = useUsageQuery({
    queryId: DATABRICKS_QUERY_IDS.teamVectorStorage,
    teamId,
    projectId,
    period,
    componentPrefix,
  });
  return data?.map(([_teamId, ds, vectorStorage]) => ({
    ds,
    value: Number(vectorStorage),
  }));
}

export function useUsageTeamVectorBandwidthPerDay(
  teamId: number,
  projectId: number | null,
  period: DateRange | null,
  componentPrefix: string | null,
) {
  const { data } = useUsageQuery({
    queryId: DATABRICKS_QUERY_IDS.teamVectorBandwidth,
    teamId,
    projectId,
    period,
    componentPrefix,
  });
  return data?.map(([_teamId, ds, ingressSize, egressSize]) => ({
    ds,
    metrics: [
      { tag: "ingress", value: Number(ingressSize) },
      {
        tag: "egress",
        value: Number(egressSize),
      },
    ],
  }));
}

export function useUsageTeamDatabaseStoragePerDay(
  teamId: number,
  projectId: number | null,
  period: DateRange | null,
  componentPrefix: string | null,
): DailyPerTagMetrics[] | undefined {
  const { data } = useUsageQuery({
    queryId: DATABRICKS_QUERY_IDS.teamDatabaseStorage,
    teamId,
    projectId,
    period,
    componentPrefix,
  });
  return data?.map(([_teamId, ds, documentStorage, indexStorage]) => ({
    ds,
    metrics: [
      { tag: "document", value: Number(documentStorage) },
      {
        tag: "index",
        value: Number(indexStorage),
      },
    ],
  }));
}

export function useUsageTeamActionComputeDaily(
  teamId: number,
  projectId: number | null,
  period: DateRange | null,
  componentPrefix: string | null,
): DailyMetric[] | undefined {
  const { data } = useUsageQuery({
    queryId: DATABRICKS_QUERY_IDS.teamActionCompute,
    teamId,
    projectId,
    period,
    componentPrefix,
  });
  return data?.map(([_teamId, ds, valueGbS]) => {
    const valueGbHour = Number(valueGbS) / 60 / 60;
    return {
      ds,
      value: valueGbHour,
    };
  });
}

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
export function useUsageTeamDailyCallsByTag(
  teamId: number,
  projectId: number | null,
  period: DateRange | null,
  componentPrefix: string | null,
) {
  const { data: functionData } = useUsageQuery({
    queryId: DATABRICKS_QUERY_IDS.teamFunctionCalls,
    teamId,
    projectId,
    period,
    componentPrefix,
  });

  const { data: storageData } = useUsageQuery({
    queryId: DATABRICKS_QUERY_IDS.teamStorageCalls,
    teamId,
    projectId,
    period,
    componentPrefix,
  });

  // Start with functionData
  const metrics = functionData?.map(
    ([
      _teamId,
      ds,
      cachedQueries,
      uncachedQueries,
      mutations,
      actions,
      httpActions,
    ]) => ({
      ds,
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
  const storageDataByDs = (storageData || []).reduce(
    (acc, [_teamId, ds, storageCalls]) => {
      acc[ds] = Number(storageCalls);
      return acc;
    },
    {} as Record<string, number>,
  );
  for (const metric of metrics || []) {
    const storageCalls = storageDataByDs[metric.ds];
    if (storageCalls) {
      metric.metrics.push({ tag: "storage_api", value: storageCalls });
      delete storageDataByDs[metric.ds];
    }
  }

  return metrics;
}

export function useUsageTeamStoragePerDay(
  teamId: number,
  projectId: number | null,
  period: DateRange | null,
  componentPrefix: string | null,
): DailyPerTagMetrics[] | undefined {
  const { data } = useUsageQuery({
    queryId: DATABRICKS_QUERY_IDS.teamFileStorage,
    teamId,
    projectId,
    period,
    componentPrefix,
  });
  return data?.map(
    ([_teamId, ds, _totalFileSize, userFileSize, cloudBackupSize]) => ({
      ds,
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

export function useUsageTeamStorageThroughputDaily(
  teamId: number,
  projectId: number | null,
  period: DateRange | null,
  componentPrefix: string | null,
) {
  const { data } = useUsageQuery({
    queryId: DATABRICKS_QUERY_IDS.teamFileBandwidth,
    teamId,
    projectId,
    period,
    componentPrefix,
  });
  return data?.map(
    ([
      _teamId,
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
      metrics: [
        { tag: "servingIngress", value: Number(servingIngressSize) },
        {
          tag: "servingEgress",
          value: Number(servingEgressSize),
        },
        {
          tag: "userFunctionIngress",
          value: Number(userFunctionIngressSize),
        },
        {
          tag: "userFunctionEgress",
          value: Number(userFunctionEgressSize),
        },
        {
          tag: "cloudBackup",
          value: Number(cloudBackupSize),
        },
        {
          tag: "cloudRestore",
          value: Number(cloudRestoreSize),
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
